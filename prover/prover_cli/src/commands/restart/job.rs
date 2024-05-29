use clap::Args as ClapArgs;
use prover_dal::{Connection, ConnectionPool, Prover, ProverDal};
use zksync_basic_types::{
    basic_fri_types::AggregationRound::{self, *},
    L1BatchNumber,
};

use crate::cli::ProverCLIConfig;
use crate::commands::restart::batch;

// TODO:
// - Invalidation logic: when I reset a job, I need all jobs depending on its
//   data to be reset as well. The dependency is determined by the aggregation
//   round, the job type, the batch number, and the circuit id.
//   Specifically:
//   - Rounds BasicCircuits and LeafAggregation are filtered by circuit id.
//   - Rounds NodeAggregation and WitnessGeneration restart the whole next
//     aggregation round for the batch.
// - I probably need to write a few new queries in the DAL to support this.
// - We can probably also check the jobs correspond to the right round by id.

#[derive(ClapArgs)]
#[clap(group(
        clap::ArgGroup::new("component")
            .required(true)
            .args(&["prover_job", "witness_job", "compressor_job"]),
        ))]
pub struct Args {
    #[clap(short, long, required(true))]
    id: u32,
    #[clap(value_enum, short, long = "witness-generator-job")]
    witness_job: Option<AggregationRound>,
    #[clap(value_enum, short, long)]
    prover_job: Option<AggregationRound>,
    #[clap(short, long = "compressor-job")]
    compressor_job: Option<bool>,
}

pub async fn run(args: Args, config: ProverCLIConfig) -> anyhow::Result<()> {
    let connection_pool =
        ConnectionPool::<Prover>::singleton(config.db_url)
            .build()
            .await?;
    let mut conn = connection_pool.connection().await?;
    if let Some(round) = args.witness_job {
        return restart_witness_job(round, args.id, &mut conn).await;
    }
    if let Some(round) = args.prover_job {
        return restart_prover_job(round, args.id, &mut conn).await;
    }
    if let Some(_) = args.compressor_job {
        return restart_compressor_job(args.id, &mut conn).await;
    }
    // This case is filtered by the required argument group.
    unreachable!()
}

async fn restart_prover_job(round: AggregationRound, id: u32, conn: &mut Connection<'_, Prover>) -> Result<(), anyhow::Error> {
    if matches!(round, RecursionTip|Scheduler) {
        return batch::restart_from_prover_jobs_in_aggregation_round(round, L1BatchNumber::from(id), conn).await;
    }
    let mut dal = conn.fri_prover_jobs_dal();
    let (batch, circuit_id) = dal.restart_prover_job_fri(round, id).await?;
    if let Some(next_round) = round.next() {
        dal.delete_data_for_circuit_from_round(batch, next_round, circuit_id).await?;
    }
    Ok(())
}

async fn restart_witness_job(round: AggregationRound, id: u32, conn: &mut Connection<'_, Prover>) -> Result<(), anyhow::Error> {
    if matches!(round, RecursionTip|Scheduler) {
        return batch::restart_from_aggregation_round(round, L1BatchNumber::from(id), conn).await;
    }
    todo!();
    //conn.fri_witness_generator_dal().mark_witness_job(id, "queued").await

}

async fn restart_compressor_job(id: u32, conn: &mut Connection<'_, Prover>) -> Result<(), anyhow::Error> {
    batch::restart_compressor(L1BatchNumber::from(id), conn).await
}