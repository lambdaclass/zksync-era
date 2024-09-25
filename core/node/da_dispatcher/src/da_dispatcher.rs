use std::{collections::HashSet, future::Future, sync::Arc, time::Duration};

use anyhow::Context;
use chrono::Utc;
use futures::future::join_all;
use rand::Rng;
use tokio::sync::{mpsc, watch::Receiver};
use zksync_config::DADispatcherConfig;
use zksync_da_client::{
    types::{DAError, InclusionData},
    DataAvailabilityClient,
};
use zksync_dal::{ConnectionPool, Core, CoreDal};
use zksync_types::L1BatchNumber;

use crate::metrics::METRICS;

#[derive(Debug)]
pub struct DataAvailabilityDispatcher {
    client: Box<dyn DataAvailabilityClient>,
    pool: ConnectionPool<Core>,
    config: DADispatcherConfig,
    request_semaphore: Arc<tokio::sync::Semaphore>,
}

// TODO: this function might be movable to DataAvailabilityDispatcher
async fn dispatch_batches(
    pool: ConnectionPool<Core>,
    config: DADispatcherConfig,
    client: Box<dyn DataAvailabilityClient>,
    stop_receiver: Receiver<bool>,
    semaphore: Arc<tokio::sync::Semaphore>,
) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel(100);

    let stop_receiver_clone = stop_receiver.clone();
    let pool_clone = pool.clone();
    let config_clone = config.clone();
    let pending_blobs_reader = tokio::spawn(async move {
        // Used to avoid sending the same batch multiple times
        let mut pending_batches = HashSet::new();
        loop {
            if *stop_receiver_clone.borrow() {
                break;
            }

            let mut conn = pool_clone.connection_tagged("da_dispatcher").await.unwrap();
            let batches = conn
                .data_availability_dal()
                .get_ready_for_da_dispatch_l1_batches(config_clone.max_rows_to_dispatch() as usize)
                .await
                .unwrap();
            drop(conn);
            for batch in batches {
                if pending_batches.contains(&batch.l1_batch_number.0) {
                    continue;
                }
                pending_batches.insert(batch.l1_batch_number.0);
                METRICS.blobs_pending_dispatch.inc_by(1);
                tx.send(batch).await.unwrap();
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let pending_blobs_sender = tokio::spawn(async move {
        let mut spawned_requests = vec![];
        loop {
            if *stop_receiver.borrow() {
                break;
            }

            let batch = match rx.recv().await {
                Some(batch) => batch,
                None => continue, // TODO: why does this happen?
            };

            // Block until we can send the request
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            let client = client.clone();
            let pool = pool.clone();
            let config = config.clone();
            let request = tokio::spawn(async move {
                let _permit = permit; // move permit into scope
                let dispatch_latency = METRICS.blob_dispatch_latency.start();
                let dispatch_response = retry(config.max_retries(), batch.l1_batch_number, || {
                    client.dispatch_blob(batch.l1_batch_number.0, batch.pubdata.clone())
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

                let sent_at = Utc::now().naive_utc();

                let mut conn = pool.connection_tagged("da_dispatcher").await?;
                conn.data_availability_dal()
                    .insert_l1_batch_da(
                        batch.l1_batch_number,
                        dispatch_response.blob_id.as_str(),
                        sent_at,
                    )
                    .await?;
                drop(conn);

                METRICS
                    .last_dispatched_l1_batch
                    .set(batch.l1_batch_number.0 as usize);
                METRICS.blob_size.observe(batch.pubdata.len());
                METRICS.blobs_dispatched.inc_by(1);
                METRICS.blobs_pending_dispatch.dec_by(1);
                tracing::info!(
                    "Dispatched a DA for batch_number: {}, pubdata_size: {}, dispatch_latency: {dispatch_latency_duration:?}",
                    batch.l1_batch_number,
                    batch.pubdata.len(),
                );

                Ok::<(), anyhow::Error>(())
            });
            spawned_requests.push(request);
        }
        join_all(spawned_requests).await;
    });

    let results = join_all(vec![pending_blobs_reader, pending_blobs_sender]).await;
    for result in results {
        result?;
    }
    Ok(())
}

// TODO: this function might be movable to DataAvailabilityDispatcher
async fn inclusion_poller(
    pool: ConnectionPool<Core>,
    config: DADispatcherConfig,
    client: Box<dyn DataAvailabilityClient>,
    stop_receiver: Receiver<bool>,
    semaphore: Arc<tokio::sync::Semaphore>,
) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel(100);

    let stop_receiver_clone = stop_receiver.clone();
    let pool_clone = pool.clone();
    let pending_inclusion_reader = tokio::spawn(async move {
        let mut pending_inclusions = HashSet::new();
        loop {
            if *stop_receiver_clone.borrow() {
                break;
            }

            let mut conn = pool_clone.connection_tagged("da_dispatcher").await.unwrap();
            // TODO: this query might always return the same blob if the blob is not included
            // we should probably change the query to return all blobs that are not included
            let blob_info = conn
                .data_availability_dal()
                .get_first_da_blob_awaiting_inclusion()
                .await
                .unwrap();
            drop(conn);

            let Some(blob_info) = blob_info else {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            };

            if pending_inclusions.contains(&blob_info.blob_id) {
                continue;
            }
            pending_inclusions.insert(blob_info.blob_id.clone());
            tx.send(blob_info).await.unwrap();
        }
    });

    let pending_inclusion_sender = tokio::spawn(async move {
        let mut spawned_requests = vec![];
        loop {
            if *stop_receiver.borrow() {
                break;
            }
            let blob_info = rx.recv().await.unwrap();

            // Block until we can send the request
            let permit = semaphore.clone().acquire_owned().await.unwrap();

            let client = client.clone();
            let pool = pool.clone();
            let config = config.clone();
            let request = tokio::spawn(async move {
                let _permit = permit; // move permit into scope
                let inclusion_data = if config.use_dummy_inclusion_data() {
                    client
                        .get_inclusion_data(blob_info.blob_id.as_str())
                        .await
                        .with_context(|| {
                            format!(
                                "failed to get inclusion data for blob_id: {}, batch_number: {}",
                                blob_info.blob_id, blob_info.l1_batch_number
                            )
                        })?
                } else {
                    // if the inclusion verification is disabled, we don't need to wait for the inclusion
                    // data before committing the batch, so simply return an empty vector
                    Some(InclusionData { data: vec![] })
                };

                let Some(inclusion_data) = inclusion_data else {
                    return Ok(());
                };

                let mut conn = pool.connection_tagged("da_dispatcher").await?;
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

                Ok::<(), anyhow::Error>(())
            });
            spawned_requests.push(request);
        }
        join_all(spawned_requests).await;
    });

    let results = join_all(vec![pending_inclusion_reader, pending_inclusion_sender]).await;
    for result in results {
        result?;
    }
    Ok(())
}

impl DataAvailabilityDispatcher {
    pub fn new(
        pool: ConnectionPool<Core>,
        config: DADispatcherConfig,
        client: Box<dyn DataAvailabilityClient>,
    ) -> Self {
        let request_semaphore = Arc::new(tokio::sync::Semaphore::new(100));
        Self {
            pool,
            config,
            client,
            request_semaphore,
        }
    }

    pub async fn run(self, mut stop_receiver: Receiver<bool>) -> anyhow::Result<()> {
        let subtasks = futures::future::join(
            async {
                if let Err(err) = dispatch_batches(
                    self.pool.clone(),
                    self.config.clone(),
                    self.client.clone(),
                    stop_receiver.clone(),
                    self.request_semaphore.clone(),
                )
                .await
                {
                    tracing::error!("dispatch error {err:?}");
                }
            },
            async {
                if let Err(err) = inclusion_poller(
                    self.pool.clone(),
                    self.config.clone(),
                    self.client.clone(),
                    stop_receiver.clone(),
                    self.request_semaphore.clone(),
                )
                .await
                {
                    tracing::error!("poll_for_inclusion error {err:?}");
                }
            },
        );

        tokio::select! {
            _ = subtasks => {},
        }
        Ok(())
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
                tracing::warn!(%err, "Failed DA dispatch request {retries}/{max_retries} for batch {batch_number}, retrying in {} milliseconds.", sleep_duration.as_millis());
                tokio::time::sleep(sleep_duration).await;

                backoff_secs = (backoff_secs * 2).min(128); // cap the back-off at 128 seconds
            }
        }
    }
}
