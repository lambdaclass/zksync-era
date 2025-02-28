use std::{future::Future, sync::Arc, time::Duration};

use anyhow::Context;
use chrono::Utc;
use rand::Rng;
use tokio::sync::watch::Receiver;
use zksync_config::{ContractsConfig, DADispatcherConfig};
use zksync_da_client::{
    types::{DAError, InclusionData},
    DataAvailabilityClient,
};
use zksync_dal::{ConnectionPool, Core, CoreDal};
use zksync_eth_client::{
    clients::{DynClient, L1},
    EthInterface,
};
use zksync_types::{
    ethabi, l2_to_l1_log::L2ToL1Log, web3::CallRequest, Address, L1BatchNumber, H256,
};

use crate::metrics::METRICS;

#[derive(Debug, Clone)]
pub struct DataAvailabilityDispatcher {
    client: Box<dyn DataAvailabilityClient>,
    pool: ConnectionPool<Core>,
    config: DADispatcherConfig,
    contracts_config: ContractsConfig,
    settlement_layer_client: Box<DynClient<L1>>,

    transitional_l2_da_validator_address: Option<Address>, // set only if inclusion_verification_transition_enabled is true
}

impl DataAvailabilityDispatcher {
    pub fn new(
        pool: ConnectionPool<Core>,
        config: DADispatcherConfig,
        client: Box<dyn DataAvailabilityClient>,
        contracts_config: ContractsConfig,
        settlement_layer_client: Box<DynClient<L1>>,
    ) -> Self {
        Self {
            pool,
            config,
            client,
            contracts_config,
            settlement_layer_client,

            transitional_l2_da_validator_address: None,
        }
    }

    pub async fn run(mut self, mut stop_receiver: Receiver<bool>) -> anyhow::Result<()> {
        self.check_for_misconfiguration().await?;
        let self_arc = Arc::new(self.clone());

        let mut stop_receiver_dispatch = stop_receiver.clone();
        let mut stop_receiver_poll_for_inclusion = stop_receiver.clone();

        let dispatch_task = tokio::spawn(async move {
            loop {
                if *stop_receiver_dispatch.borrow() {
                    break;
                }

                if let Err(err) = self_arc.dispatch().await {
                    tracing::error!("dispatch error {err:?}");
                }

                if tokio::time::timeout(
                    self_arc.config.polling_interval(),
                    stop_receiver_dispatch.changed(),
                )
                .await
                .is_ok()
                {
                    break;
                }
            }
        });

        let inclusion_task = tokio::spawn(async move {
            loop {
                if *stop_receiver_poll_for_inclusion.borrow() {
                    break;
                }

                if let Err(err) = self.poll_for_inclusion().await {
                    tracing::error!("poll_for_inclusion error {err:?}");
                }

                if tokio::time::timeout(
                    self.config.polling_interval(),
                    stop_receiver_poll_for_inclusion.changed(),
                )
                .await
                .is_ok()
                {
                    break;
                }
            }
        });

        tokio::select! {
            _ = dispatch_task => {},
            _ = inclusion_task => {},
            _ = stop_receiver.changed() => {},
        }

        tracing::info!("Stop signal received, da_dispatcher is shutting down");
        Ok(())
    }

    /// Dispatches the blobs to the data availability layer, and saves the blob_id in the database.
    async fn dispatch(&self) -> anyhow::Result<()> {
        let mut conn = self.pool.connection_tagged("da_dispatcher").await?;
        let batches = conn
            .data_availability_dal()
            .get_ready_for_da_dispatch_l1_batches(self.config.max_rows_to_dispatch() as usize)
            .await?;
        drop(conn);

        for batch in &batches {
            let dispatch_latency = METRICS.blob_dispatch_latency.start();
            let dispatch_response = retry(self.config.max_retries(), batch.l1_batch_number, || {
                self.client
                    .dispatch_blob(batch.l1_batch_number.0, batch.pubdata.clone())
            })
            .await
            .with_context(|| {
                format!(
                    "failed to dispatch a blob with batch_number: {}, pubdata_len: {}",
                    batch.l1_batch_number,
                    batch.pubdata.len()
                )
            })?;
            let dispatch_latency_duration = dispatch_latency.observe();

            let sent_at = Utc::now();

            let mut conn = self.pool.connection_tagged("da_dispatcher").await?;
            conn.data_availability_dal()
                .insert_l1_batch_da(
                    batch.l1_batch_number,
                    dispatch_response.blob_id.as_str(),
                    sent_at.naive_utc(),
                    find_l2_da_validator_address(batch.system_logs.as_slice())?,
                )
                .await?;
            drop(conn);

            METRICS
                .last_dispatched_l1_batch
                .set(batch.l1_batch_number.0 as usize);
            METRICS.blob_size.observe(batch.pubdata.len());
            METRICS.blobs_dispatched.inc_by(1);
            METRICS.sealed_to_dispatched_lag.observe(
                sent_at
                    .signed_duration_since(batch.sealed_at)
                    .to_std()
                    .context("sent_at has to be higher than sealed_at")?,
            );
            tracing::info!(
                "Dispatched a DA for batch_number: {}, pubdata_size: {}, dispatch_latency: {dispatch_latency_duration:?}",
                batch.l1_batch_number,
                batch.pubdata.len(),
            );
        }

        // We don't need to report this metric every iteration, only once when the balance is changed
        if !batches.is_empty() {
            let client_arc = Arc::new(self.client.clone_boxed());

            tokio::spawn(async move {
                let balance = client_arc
                    .balance()
                    .await
                    .context("Unable to retrieve DA operator balance");

                match balance {
                    Ok(balance) => {
                        METRICS.operator_balance.set(balance);
                    }
                    Err(err) => {
                        tracing::error!("{err}")
                    }
                }
            });
        }

        Ok(())
    }

    /// Polls the data availability layer for inclusion data, and saves it in the database.
    async fn poll_for_inclusion(&self) -> anyhow::Result<()> {
        if self.config.inclusion_verification_transition_enabled() {
            if let Some(l2_da_validator) = self.transitional_l2_da_validator_address {
                // Setting dummy inclusion data to the batches with the old L2 DA validator is necessary
                // for the transition process. We want to avoid the situation when the batch was sealed
                // but not dispatched to DA layer before transition, and then it will have an inclusion
                // data that is meant to be used with the new L2 DA validator. This will cause the
                // mismatch during the CommitBatches transaction. To avoid that we need to commit that
                // batch with dummy inclusion data during transition.
                let mut conn = self.pool.connection_tagged("da_dispatcher").await?;
                conn.data_availability_dal()
                    .set_dummy_inclusion_data_for_old_batches(l2_da_validator)
                    .await?;
            }

            return Ok(());
        }

        let mut conn = self.pool.connection_tagged("da_dispatcher").await?;
        let blob_info = conn
            .data_availability_dal()
            .get_first_da_blob_awaiting_inclusion()
            .await?;
        drop(conn);

        let Some(blob_info) = blob_info else {
            return Ok(());
        };

        let inclusion_data = if self.config.use_dummy_inclusion_data() {
            Some(InclusionData { data: vec![] })
        } else {
            self.client
                .get_inclusion_data(blob_info.blob_id.as_str())
                .await
                .with_context(|| {
                    format!(
                        "failed to get inclusion data for blob_id: {}, batch_number: {}",
                        blob_info.blob_id, blob_info.l1_batch_number
                    )
                })?
        };

        let Some(inclusion_data) = inclusion_data else {
            return Ok(());
        };

        let mut conn = self.pool.connection_tagged("da_dispatcher").await?;
        conn.data_availability_dal()
            .save_l1_batch_inclusion_data(
                L1BatchNumber(blob_info.l1_batch_number.0),
                inclusion_data.data.as_slice(),
            )
            .await?;
        drop(conn);

        let inclusion_latency = Utc::now().signed_duration_since(blob_info.sent_at);
        if let Ok(latency) = inclusion_latency.to_std() {
            METRICS.inclusion_latency.observe(latency);
        }
        METRICS
            .last_included_l1_batch
            .set(blob_info.l1_batch_number.0 as usize);
        METRICS.blobs_included.inc_by(1);

        tracing::info!(
            "Received an inclusion data for a batch_number: {}, inclusion_latency_seconds: {}",
            blob_info.l1_batch_number,
            inclusion_latency.num_seconds()
        );

        Ok(())
    }

    async fn check_for_misconfiguration(&mut self) -> anyhow::Result<()> {
        if let Some(no_da_validator) = self.contracts_config.no_da_validium_l1_validator_addr {
            if self.config.use_dummy_inclusion_data() {
                let l1_da_validator_address = self.fetch_l1_da_validator_address().await?;

                if l1_da_validator_address != no_da_validator {
                    anyhow::bail!(
                        "Dummy inclusion data is enabled, but not the NoDAValidator is used: {:?} != {:?}",
                        l1_da_validator_address, no_da_validator
                    )
                }
            }
        }

        if self.config.inclusion_verification_transition_enabled() {
            self.transitional_l2_da_validator_address = Some(
                self.contracts_config
                    .l2_da_validator_addr
                    .context("L2 DA validator address is not set")?,
            );
        }

        Ok(())
    }

    async fn fetch_l1_da_validator_address(&self) -> anyhow::Result<Address> {
        let signature = ethabi::short_signature("getDAValidatorPair", &[]);
        let response = self
            .settlement_layer_client
            .call_contract_function(
                CallRequest {
                    data: Some(signature.into()),
                    to: Some(self.contracts_config.diamond_proxy_addr),
                    ..CallRequest::default()
                },
                None,
            )
            .await
            .context("Failed to call the DA validator getter")?;

        let validators = ethabi::decode(
            &[ethabi::ParamType::Address, ethabi::ParamType::Address],
            response.0.as_slice(),
        )
        .context("Failed to decode the DA validator address")?;

        validators[0]
            .clone()
            .into_address()
            .context("Failed to convert DA validator address from Token")
    }
}

async fn retry<T, Fut, F>(
    max_retries: u16,
    batch_number: L1BatchNumber,
    mut f: F,
) -> Result<T, DAError>
where
    Fut: Future<Output = Result<T, DAError>>,
    F: FnMut() -> Fut,
{
    let mut retries = 1;
    let mut backoff_secs = 1;
    loop {
        match f().await {
            Ok(result) => {
                METRICS.dispatch_call_retries.observe(retries as usize);
                return Ok(result);
            }
            Err(err) => {
                if !err.is_retriable() || retries > max_retries {
                    return Err(err);
                }

                retries += 1;
                let sleep_duration = Duration::from_secs(backoff_secs)
                    .mul_f32(rand::thread_rng().gen_range(0.8..1.2));
                tracing::warn!(
                    %err,
                    "Failed DA dispatch request {retries}/{} for batch {batch_number}, retrying in {} milliseconds.",
                    max_retries+1,
                    sleep_duration.as_millis()
                );
                tokio::time::sleep(sleep_duration).await;

                backoff_secs = (backoff_secs * 2).min(128); // cap the back-off at 128 seconds
            }
        }
    }
}

pub fn find_l2_da_validator_address(system_logs: &[L2ToL1Log]) -> anyhow::Result<Address> {
    Ok(system_logs
        .iter()
        .find(|log| {
            log.key
                == H256::from_low_u64_be(u64::from(
                    zksync_system_constants::L2_DA_VALIDATOR_OUTPUT_HASH_KEY,
                ))
        })
        .context("L2 DA validator address log is missing")?
        .value
        .into())
}
