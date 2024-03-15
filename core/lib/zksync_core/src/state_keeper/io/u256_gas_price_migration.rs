use std::{
    convert::Infallible,
    future::{self, Future},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context as _;
use multivm::interface::{Halt, L1BatchEnv, SystemEnv};
use tokio::sync::watch;
use zksync_dal::ConnectionPool;
use zksync_types::{
    block::MiniblockExecutionData,
    l2::TransactionType,
    protocol_version::{ProtocolUpgradeTx, ProtocolVersionId},
    storage_writes_deduplicator::StorageWritesDeduplicator,
    L1BatchNumber, Transaction,
};

use super::{
    batch_executor::{BatchExecutor, BatchExecutorHandle, TxExecutionResult},
    extractors,
    io::{MiniblockParams, PendingBatchData, StateKeeperIO},
    metrics::{AGGREGATION_METRICS, KEEPER_METRICS, L1_BATCH_METRICS},
    seal_criteria::{ConditionalSealer, SealData, SealResolution},
    types::ExecutionMetricsForCriteria,
    updates::UpdatesManager,
};
use crate::{
    gas_tracker::gas_count_from_writes,
    state_keeper::{io::fee_address_migration, metrics::BATCH_TIP_METRICS},
};
use zksync_types::MiniblockNumber;
/// Runs the migration for non-pending miniblocks. Should be run as a background task.
pub(crate) async fn migrate_miniblocks(
    pool: ConnectionPool,
    last_miniblock: MiniblockNumber,
    stop_receiver: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    // `migrate_miniblocks_inner` assumes that miniblocks start from the genesis (i.e., no snapshot recovery).
    // Since snapshot recovery is later that the fee address migration in terms of code versioning,
    // the migration is always no-op in case of snapshot recovery; all miniblocks added after recovery are guaranteed
    // to have their fee address set.
    let mut storage = pool.access_storage_tagged("state_keeper").await?;
    if storage
        .snapshot_recovery_dal()
        .get_applied_snapshot_status()
        .await?
        .is_some()
    {
        tracing::info!("Detected snapshot recovery; gas price migration is skipped as no-op");
        return Ok(());
    }

    drop(storage);

    // let MigrationOutput {
    //     miniblocks_affected,
    // migrate_miniblocks_inner(
    //     pool,
    //     last_miniblock,
    //     100_000,
    //     Duration::from_secs(1),
    //     stop_receiver,
    // )
    // .await?;

    // tracing::info!("Finished fee address migration with {miniblocks_affected} affected miniblocks");
    Ok(())
}

async fn migrate_miniblocks_inner(
    pool: ConnectionPool,
    last_miniblock: MiniblockNumber,
    chunk_size: u32,
    sleep_interval: Duration,
    stop_receiver: watch::Receiver<bool>,
) -> anyhow::Result<u64> {
    anyhow::ensure!(chunk_size > 0, "Chunk size must be positive");
    
    let mut storage = pool.access_storage_tagged("state_keeper").await?;
    drop(storage);

    let mut chunk_start = MiniblockNumber(0);
    let mut miniblocks_affected = 0;

    tracing::info!(
        "Migrating `fee_account_address` for miniblocks {chunk_start}..={last_miniblock} \
         in chunks of {chunk_size} miniblocks"
    );
    while chunk_start <= last_miniblock {
        let chunk_end = last_miniblock.min(chunk_start + chunk_size - 1);
        let chunk = chunk_start..=chunk_end;

        let mut storage = pool.access_storage_tagged("state_keeper").await?;
        // let is_chunk_migrated = is_fee_address_migrated(&mut storage, chunk_start).await?;
        if let Some(is_miniblock_migrated) = storage
            .blocks_dal()
            .is_miniblock_price_migrated_to_u256(chunk_start)
            .await?
        {
            if is_miniblock_migrated {
                tracing::debug!("`fee_account_address` is migrated for chunk {chunk:?}");
            } else {
                tracing::debug!("Migrating `fee_account_address` for miniblocks chunk {chunk:?}");

                #[allow(deprecated)]
                let rows_affected = storage
                    .blocks_dal()
                    .copy_fee_account_address_for_miniblocks(chunk.clone())
                    .await
                    .with_context(|| format!("Failed migrating miniblocks chunk {chunk:?}"))?;
                tracing::debug!("Migrated {rows_affected} miniblocks in chunk {chunk:?}");
                miniblocks_affected += rows_affected;
            }
            drop(storage);

            if *stop_receiver.borrow() {
                tracing::info!("Stop signal received; fee address migration shutting down");
                Ok(1_u64)
                // return Ok(MigrationOutput {
                //     miniblocks_affected,
                // });
            }
            chunk_start = chunk_end + 1;

            if !is_miniblock_migrated {
                tokio::time::sleep(sleep_interval).await;
            }
        } else {
            tracing::error!("Wrong miniblock number given for price migration");
        }
    }

    Ok(1)
    // Ok(MigrationOutput {
    //     miniblocks_affected,
    // })
}
