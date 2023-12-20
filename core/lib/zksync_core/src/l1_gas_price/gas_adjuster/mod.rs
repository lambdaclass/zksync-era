//! This module determines the fees to pay in txs containing blocks submitted to the L1.

use ::metrics::atomics::AtomicU64;
use tokio::sync::watch;

use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use zksync_config::GasAdjusterConfig;
use zksync_eth_client::{types::Error, EthInterface};

pub mod bounded_gas_adjuster;
mod metrics;
#[cfg(test)]
mod tests;

use self::metrics::METRICS;
use super::{L1GasPriceProvider, L1TxParamsProvider};

use serde::Deserialize;
use serde::Serialize;
#[derive(Deserialize, Serialize, Debug)]
struct EthValue {
    eth: serde_json::value::Number,
}
#[derive(Deserialize, Serialize, Debug)]
struct Request {
    dai: EthValue,
}

/// Dedicated tether <-> eth value fetcher.
#[derive(Debug)]
struct ERC20Fetcher {
    value: u64,
    url: String,
}

impl ERC20Fetcher {
    async fn new() -> Self {
        let url = "https://api.coingecko.com/api/v3/simple/price?x_cg_demo_api_key=CG-FEgodj8AJN55Va4c6uKPUWLe&ids=dai&vs_currencies=eth".to_string();
        let value = Self::fetch_it(&url).await;
        Self {
            value: Self::erc20_value_in_wei(&value),
            url,
        }
    }

    async fn fetch_it(url: &str) -> String {
        let response = reqwest::get(url)
            .await
            .expect("Failed request for ERC-20")
            .json::<Request>()
            .await
            .unwrap();
        return response.dai.eth.to_string();
    }

    fn erc20_value_in_wei(value: &str) -> u64 {
        let vec: Vec<&str> = value.split(".").collect();
        let whole_part: u64 = u64::from_str_radix(vec.first().unwrap(), 10).unwrap();
        let whole_part_to_wei = Self::to_wei(whole_part, 0_u32);
        let decimal_length = vec.last().unwrap().len() as u32;
        let decimal_part = u64::from_str_radix(vec.last().unwrap(), 10).unwrap();
        let decimal_part_to_wei = Self::to_wei(decimal_part, decimal_length);
        return whole_part_to_wei + decimal_part_to_wei;
    }

    fn to_wei(in_eth: u64, modifier: u32) -> u64 {
        in_eth * 10_u64.pow(18_u32 - modifier)
    }
    async fn update_value(&mut self) {
        let new_value = Self::fetch_it(&self.url).await;

        self.value = Self::erc20_value_in_wei(&new_value);
    }
}

/// This component keeps track of the median base_fee from the last `max_base_fee_samples` blocks.
/// It is used to adjust the base_fee of transactions sent to L1.
#[derive(Debug)]
pub struct GasAdjuster<E> {
    pub(super) statistics: GasStatistics,
    pub(super) config: GasAdjusterConfig,
    eth_client: E,
    erc_20_fetcher: ERC20Fetcher,
    erc_20_value_in_wei: AtomicU64,
}

impl<E: EthInterface> GasAdjuster<E> {
    pub async fn new(eth_client: E, config: GasAdjusterConfig) -> Result<Self, Error> {
        // Subtracting 1 from the "latest" block number to prevent errors in case
        // the info about the latest block is not yet present on the node.
        // This sometimes happens on Infura.
        let current_block = eth_client
            .block_number("gas_adjuster")
            .await?
            .as_usize()
            .saturating_sub(1);
        let history = eth_client
            .base_fee_history(current_block, config.max_base_fee_samples, "gas_adjuster")
            .await?;
        let erc_20_fetcher = ERC20Fetcher::new().await;
        let erc_20_value = erc_20_fetcher.value.clone();
        Ok(Self {
            statistics: GasStatistics::new(config.max_base_fee_samples, current_block, &history),
            eth_client,
            config,
            erc_20_fetcher,
            erc_20_value_in_wei: AtomicU64::new(erc_20_value),
        })
    }

    /// Performs an actualization routine for `GasAdjuster`.
    /// This method is intended to be invoked periodically.
    pub async fn keep_updated(&self) -> Result<(), Error> {
        // Subtracting 1 from the "latest" block number to prevent errors in case
        // the info about the latest block is not yet present on the node.
        // This sometimes happens on Infura.
        let current_block = self
            .eth_client
            .block_number("gas_adjuster")
            .await?
            .as_usize()
            .saturating_sub(1);

        let last_processed_block = self.statistics.last_processed_block();

        if current_block > last_processed_block {
            // Report the current price to be gathered by the statistics module.
            let history = self
                .eth_client
                .base_fee_history(
                    current_block,
                    current_block - last_processed_block,
                    "gas_adjuster",
                )
                .await?;

            METRICS
                .current_base_fee_per_gas
                .set(*history.last().unwrap());
            self.statistics.add_samples(&history);
        }
        let new_value =
            ERC20Fetcher::fetch_it("https://api.coingecko.com/api/v3/simple/price?x_cg_demo_api_key=CG-FEgodj8AJN55Va4c6uKPUWLe&ids=dai&vs_currencies=eth").await;
        println!("Dai value in eth: {}", new_value);
        println!(
            "Dai value in wei: {}",
            ERC20Fetcher::erc20_value_in_wei(&new_value)
        );
        self.erc_20_value_in_wei.store(
            ERC20Fetcher::erc20_value_in_wei(&new_value),
            std::sync::atomic::Ordering::Relaxed,
        );
        println!("Gas price {}", self.estimate_effective_gas_price());
        println!("Price in erc 20: {}", self.erc_20_gas_price());
        Ok(())
    }

    pub fn erc_20_gas_price(&self) -> u64 {
        self.erc_20_value_in_wei
            .load(std::sync::atomic::Ordering::Relaxed)
            / self.estimate_effective_gas_price()
    }

    pub async fn run(self: Arc<Self>, stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
        loop {
            if *stop_receiver.borrow() {
                tracing::info!("Stop signal received, gas_adjuster is shutting down");
                break;
            }

            if let Err(err) = self.keep_updated().await {
                tracing::warn!("Cannot add the base fee to gas statistics: {}", err);
            }

            tokio::time::sleep(self.config.poll_period()).await;
        }
        Ok(())
    }
}

impl<E: EthInterface> L1GasPriceProvider for GasAdjuster<E> {
    /// Returns the sum of base and priority fee, in wei, not considering time in mempool.
    /// Can be used to get an estimate of current gas price.
    fn estimate_effective_gas_price(&self) -> u64 {
        if let Some(price) = self.config.internal_enforced_l1_gas_price {
            return price;
        }

        let effective_gas_price = self.get_base_fee(0) + self.get_priority_fee();

        (self.config.internal_l1_pricing_multiplier * effective_gas_price as f64) as u64
    }
}

impl<E: EthInterface> L1TxParamsProvider for GasAdjuster<E> {
    // This is the method where we decide how much we are ready to pay for the
    // base_fee based on the number of L1 blocks the transaction has been in the mempool.
    // This is done in order to avoid base_fee spikes (e.g. during NFT drops) and
    // smooth out base_fee increases in general.
    // In other words, in order to pay less fees, we are ready to wait longer.
    // But the longer we wait, the more we are ready to pay.
    fn get_base_fee(&self, time_in_mempool: u32) -> u64 {
        let a = self.config.pricing_formula_parameter_a;
        let b = self.config.pricing_formula_parameter_b;

        // Currently we use an exponential formula.
        // The alternative is a linear one:
        // let scale_factor = a + b * time_in_mempool as f64;
        let scale_factor = a * b.powf(time_in_mempool as f64);
        let median = self.statistics.median();
        METRICS.median_base_fee_per_gas.set(median);
        let new_fee = median as f64 * scale_factor;
        new_fee as u64
    }

    fn get_next_block_minimal_base_fee(&self) -> u64 {
        let last_block_base_fee = self.statistics.last_added_value();

        // The next block's base fee will decrease by a maximum of 12.5%.
        last_block_base_fee * 875 / 1000
    }

    // Priority fee is set to constant, sourced from config.
    // Reasoning behind this is the following:
    // High priority_fee means high demand for block space,
    // which means base_fee will increase, which means priority_fee
    // will decrease. The EIP-1559 mechanism is designed such that
    // base_fee will balance out priority_fee in such a way that
    // priority_fee will be a small fraction of the overall fee.
    fn get_priority_fee(&self) -> u64 {
        self.config.default_priority_fee_per_gas
    }
}

/// Helper structure responsible for collecting the data about recent transactions,
/// calculating the median base fee.
#[derive(Debug, Clone, Default)]
pub(super) struct GasStatisticsInner {
    samples: VecDeque<u64>,
    median_cached: u64,
    max_samples: usize,
    last_processed_block: usize,
}

impl GasStatisticsInner {
    fn new(max_samples: usize, block: usize, fee_history: &[u64]) -> Self {
        let mut statistics = Self {
            max_samples,
            samples: VecDeque::with_capacity(max_samples),
            median_cached: 0,
            last_processed_block: 0,
        };

        statistics.add_samples(fee_history);

        Self {
            last_processed_block: block,
            ..statistics
        }
    }

    fn median(&self) -> u64 {
        self.median_cached
    }

    fn last_added_value(&self) -> u64 {
        self.samples.back().copied().unwrap_or(self.median_cached)
    }

    fn add_samples(&mut self, fees: &[u64]) {
        self.samples.extend(fees);
        self.last_processed_block += fees.len();

        let extra = self.samples.len().saturating_sub(self.max_samples);
        self.samples.drain(..extra);

        let mut samples: Vec<_> = self.samples.iter().cloned().collect();
        let (_, &mut median, _) = samples.select_nth_unstable(self.samples.len() / 2);

        self.median_cached = median;
    }
}

#[derive(Debug, Default)]
pub(super) struct GasStatistics(RwLock<GasStatisticsInner>);

impl GasStatistics {
    pub fn new(max_samples: usize, block: usize, fee_history: &[u64]) -> Self {
        Self(RwLock::new(GasStatisticsInner::new(
            max_samples,
            block,
            fee_history,
        )))
    }

    pub fn median(&self) -> u64 {
        self.0.read().unwrap().median()
    }

    pub fn last_added_value(&self) -> u64 {
        self.0.read().unwrap().last_added_value()
    }

    pub fn add_samples(&self, fees: &[u64]) {
        self.0.write().unwrap().add_samples(fees)
    }

    pub fn last_processed_block(&self) -> usize {
        self.0.read().unwrap().last_processed_block
    }
}
