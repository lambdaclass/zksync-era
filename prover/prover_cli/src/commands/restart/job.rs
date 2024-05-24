use clap::Args as ClapArgs;
use prover_dal::{Connection, ConnectionPool, Prover, ProverDal};

use crate::cli::ProverCLIConfig;
use crate::prover::batch;

// TODO:
// - Invalidation logic: when I reset a job, I need all jobs depending on its
//   data to be reset as well. Thie dependency is determined by the aggregation
//   round, the job type, the batch number, and the circuit id.
//   Specifically:
//   - Rounds BasicCircuits and LeafAggregation are filtered by circuit id.
//   - Rounds NodeAggregation and WitnessGeneration restart the whole next
//     aggregation round for the batch.
// - I probably need to write a few new queries in the DAL to support this.


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(clap::ArgEnum)]
enum JobType {
    #[clap(short = "p", long = "prover-job")]
    Prover(AggregationRound),
    #[clap(short = "w", long = "witness-job")]
    Witness(AggregationRound),
    #[clap(short = "c", long = "compressor-job")]
    Compressor(bool),
}
#[derive(ClapArgs)]
pub struct Args {
    #[clap(short, long, required(true))]
    id: u32,
    #[clap(arg_enum, short, long, required(true))]
    job_type: JobType
}

pub async fn run(args: Args, config: ProverCLIConfig) -> anyhow::Result<()> {
    use AggregationRound::*;

    let connection_pool =
        ConnectionPool::<Prover>::singleton(config.db_url)
            .build()
            .await?;
    let mut conn = connection_pool.connection().await?;
    match args.job_type {
        JobType::Prover(round) => restart_prover_job(round, args.id, &mut conn).await,
        JobType::Witness(round) => restart_witness_job(round, args.id, round, &mut conn).await,
        JobType::Compressor(_) => restart_compressor_job(args.id, &mut conn).await?,
    }
    Ok(())
}

async fn restart_prover_job(round: AggregationRound, id: u32, conn: &mut Connection<'_, Prover>) {
    if matches!(round, BasicCircuits|RecursionTip|Scheduler) {
        return batch::restart_from_prover_jobs_in_aggregation_round(round, L1BatchNumber::from(id), &mut conn).await;
    }
    conn.fri_prover_jobs_dal().update_status(id, "queued").await;
}

async fn restart_witness_job(round: AggregationRound, id: u32, conn: &mut Connection<'_, Prover>) -> Result<(), anyhow::Error> {
    if matches!(round, BasicCircuits|RecursionTip|Scheduler) {
        return batch::restart_from_aggregation_round(round, L1BatchNumber::from(id), &mut conn).await;
    }
    conn.fri_witness_generator_dal().update_status(id, "queued").await;

}

async fn restart_compressor_job(id: u32, conn: &mut Connection<'_, Prover>) {
    batch::restart_compressor(L1BatchNumber::from(id), conn).await;
}