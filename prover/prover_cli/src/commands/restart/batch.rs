use anyhow::{bail, Context};
use clap::Args as ClapArgs;
use prover_dal::{
    fri_witness_generator_dal::FriWitnessJobStatus, Connection, ConnectionPool, Prover, ProverDal,
};
use zksync_types::{basic_fri_types::AggregationRound, L1BatchNumber};

use crate::cli::ProverCLIConfig;

#[derive(clap::ValueEnum, Clone, Copy)]
enum AggregationRoundProxy {
    #[clap(alias = "all-rounds")]
    All = -1,
    #[clap(name = "bwg", alias = "basic-circuits")]
    BasicCircuits = 0,
    #[clap(name = "lwg", alias = "leaf-aggregation")]
    LeafAggregation = 1,
    #[clap(name = "nwg", alias = "node-aggregation")]
    NodeAggregation = 2,
    #[clap(name = "rt", alias = "recursion-tip")]
    RecursionTip = 3,
    #[clap(name = "sched", alias = "scheduler")]
    Scheduler = 4,
}

impl From<AggregationRoundProxy> for AggregationRound {
    fn from(round: AggregationRoundProxy) -> Self {
        match round {
            AggregationRoundProxy::All|AggregationRoundProxy::BasicCircuits => AggregationRound::BasicCircuits,
            AggregationRoundProxy::LeafAggregation => AggregationRound::LeafAggregation,
            AggregationRoundProxy::NodeAggregation => AggregationRound::NodeAggregation,
            AggregationRoundProxy::RecursionTip => AggregationRound::RecursionTip,
            AggregationRoundProxy::Scheduler => AggregationRound::Scheduler,
        }
    }
}

#[derive(ClapArgs)]
#[clap(group(
        clap::ArgGroup::new("component")
            .required(true)
            .args(&["prover_jobs", "witness_jobs", "compressor"]),
        ))]
pub(crate) struct Args {
    /// Batch number to restart.
    #[clap(short = 'n', required(true))]
    batch: L1BatchNumber,
    /// Restart all prover jobs of the batch for a given round.
    #[clap(value_enum, short, long)]
    prover_jobs: Option<AggregationRoundProxy>,
    /// Restart all witness jobs of the batch for a given round.
    #[clap(value_enum, short, long = "witness-generator-jobs")]
    witness_jobs: Option<AggregationRoundProxy>,
    /// Restart the compressor job of the batch.
    #[clap(short, long = "compressor-job")]
    compressor: Option<bool>,
}

pub(crate) async fn run(args: Args, config: ProverCLIConfig) -> anyhow::Result<()> {
    let prover_connection_pool = ConnectionPool::<Prover>::singleton(config.db_url)
        .build()
        .await
        .context("failed to build a prover_connection_pool")?;
    let mut conn = prover_connection_pool.connection().await.unwrap();

    if let Some(aggregation_round) = args.prover_jobs {
        restart_from_prover_jobs_in_aggregation_round(
            aggregation_round.into(),
            args.batch,
            &mut conn,
        )
        .await?;
    }
    if let Some(aggregation_round) = args.witness_jobs {
        restart_from_aggregation_round(
            aggregation_round.into(),
            args.batch,
            &mut conn,
        )
        .await?;
    }

    Ok(())
}

async fn restart_from_prover_jobs_in_aggregation_round(
    aggregation_round: AggregationRound,
    batch_number: L1BatchNumber,
    conn: &mut Connection<'_, Prover>,
) -> anyhow::Result<()> {
    use AggregationRound::*;

    if matches!(aggregation_round, RecursionTip|Scheduler) {
        bail!("{aggregation_round:?} has no prover jobs");
    }
    let next_round = aggregation_round.next()
        .ok_or_else(|| anyhow::anyhow!("BUG: {aggregation_round:?} should have a `next` round"))?;
    restart_from_aggregation_round(next_round, batch_number, conn).await?;
    conn.fri_witness_generator_dal()
        .delete_witness_generator_data_for_batch(batch_number, next_round)
        .await
        .map_err(|e| anyhow::Error::from(e).context(format!("failed to restart prover jobs in {aggregation_round:?} round")))?;
    Ok(())
}

async fn restart_compressor(
    _batch_number: L1BatchNumber,
    _conn: &mut Connection<'_, Prover>,
) -> anyhow::Result<()> {
    // Set compressor job to queued
    Ok(())
}

async fn restart_from_aggregation_round_inner(
    aggregation_round: AggregationRound,
    batch_number: L1BatchNumber,
    conn: &mut Connection<'_, Prover>,
) -> anyhow::Result<()> {
    if let Some(next_round) = aggregation_round.next() {
        conn.fri_witness_generator_dal()
            .delete_witness_generator_data_for_batch(
                batch_number,
                next_round,
            )
            .await
            .context("failed to restart batch: fri_witness_generator_dal()")?;
    }
    match aggregation_round {
        AggregationRound::BasicCircuits => {
            conn.fri_prover_jobs_dal()
                .delete_batch_data(batch_number)
                .await
                .context("failed to delete prover jobs for batch")?;
            conn.fri_witness_generator_dal()
                .mark_witness_job(FriWitnessJobStatus::Queued, batch_number)
                .await;
        }
        AggregationRound::LeafAggregation => {
            conn.fri_prover_jobs_dal()
                .delete_batch_data_for_aggregation_round(batch_number, aggregation_round)
                .await?;
            // Mark leaf aggregation jobs as queued
        }
        AggregationRound::NodeAggregation => {
            conn.fri_prover_jobs_dal()
                .delete_batch_data_for_aggregation_round(batch_number, aggregation_round)
                .await?;
            // Mark node aggregation jobs as queued
        }
        AggregationRound::RecursionTip => {
            // Mark recursion tip job as queued
        }
        AggregationRound::Scheduler => {
            conn.fri_proof_compressor_dal()
                .delete_batch_data(batch_number)
                .await
                .context("failed to delete proof compression job for batch")?;
            // Mark scheduler job as queued
        }
    }

    Ok(())
}

async fn restart_from_aggregation_round(
    aggregation_round: AggregationRound,
    batch_number: L1BatchNumber,
    conn: &mut Connection<'_, Prover>,
) -> anyhow::Result<()> {
    let rounds: Vec<_> = std::iter::successors(
        Some(aggregation_round),
        |round| round.next(),
    )
    .collect();

    for round in rounds.into_iter().rev() {
        restart_from_aggregation_round_inner(round, batch_number, conn).await?;
    }

    Ok(())
}
