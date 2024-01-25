use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
    time::Duration,
};

use tokio::sync::watch::Receiver;
use zksync_types::fee_model::FeeParams;
use zksync_web3_decl::{
    jsonrpsee::http_client::{HttpClient, HttpClientBuilder},
    namespaces::ZksNamespaceClient,
};

use crate::{fee_model::BatchFeeModelInputProvider, l1_gas_price::gas_adjuster::erc_20_fetcher};

const SLEEP_INTERVAL: Duration = Duration::from_secs(5);

/// This structure maintains the known L1 gas price by periodically querying
/// the main node.
/// It is required since the main node doesn't only observe the current L1 gas price,
/// but also applies adjustments to it in order to smooth out the spikes.
/// The same algorithm cannot be consistently replicated on the external node side,
/// since it relies on the configuration, which may change.
#[derive(Debug)]
pub struct MainNodeFeeParamsFetcher {
    client: HttpClient,
    main_node_fee_params: RwLock<FeeParams>,
    erc20_value_in_wei: AtomicU64,
}

impl MainNodeFeeParamsFetcher {
    pub fn new(main_node_url: &str) -> Self {
        Self {
            client: Self::build_client(main_node_url),
            erc20_value_in_wei: AtomicU64::new(1u64),
            main_node_fee_params: RwLock::new(FeeParams::sensible_v1_default()),
        }
    }

    fn build_client(main_node_url: &str) -> HttpClient {
        HttpClientBuilder::default()
            .build(main_node_url)
            .expect("Unable to create a main node client")
    }

    pub async fn run(self: Arc<Self>, stop_receiver: Receiver<bool>) -> anyhow::Result<()> {
        loop {
            if *stop_receiver.borrow() {
                tracing::info!("Stop signal received, MainNodeFeeParamsFetcher is shutting down");
                break;
            }

            let main_node_fee_params = match self.client.get_fee_params().await {
                Ok(price) => price,
                Err(err) => {
                    tracing::warn!("Unable to get the gas price: {}", err);
                    // A delay to avoid spamming the main node with requests.
                    tokio::time::sleep(SLEEP_INTERVAL).await;
                    continue;
                }
            };
            *self.main_node_fee_params.write().unwrap() = main_node_fee_params;

            tokio::time::sleep(SLEEP_INTERVAL).await;

            self.erc20_value_in_wei.store(
                erc_20_fetcher::get_erc_20_value_in_wei().await,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        Ok(())
    }
}

impl BatchFeeModelInputProvider for MainNodeFeeParamsFetcher {
    fn get_fee_model_params(&self) -> FeeParams {
        *self.main_node_fee_params.read().unwrap()
    }

    fn get_erc20_conversion_rate(&self) -> u64 {
        self.erc20_value_in_wei.load(Ordering::Relaxed)
    }
}
