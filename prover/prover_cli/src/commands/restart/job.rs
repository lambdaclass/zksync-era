use clap::Args as ClapArgs;
use prover_dal::{Connection, ConnectionPool, Prover, ProverDal};

use crate::cli::ProverCLIConfig;

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
        JobType::Prover(round) => restart_prover_job(args.id, round, &mut conn).await,
        JobType::Witness(round) => restart_witness_job(round, args.id, round, &mut conn).await,
        JobType::Compressor(_) => restart_compressor(L1BatchNumber::from(args.id), &mut conn).await?,
    }
    Ok(())
}

async fn restart_prover_job(id: u32, conn: &mut Connection<'_, Prover>) {
    conn.fri_prover_jobs_dal().update_status(id, "queued").await;
}

async fn restart_witness_job(round: AggregationRound, id: u32, conn: &mut Connection<'_, Prover>) -> Result<(), anyhow::Error> {
    if matches!(round, BasicCircuits|RecursionTip|Scheduler) {
        return restart_from_aggregation_round(round, L1BatchNumber::from(args.id), &mut conn).await;
    }
    conn.witness_prover_jobs_dal().update_status(id, "queued").await;

}

async fn restart_compressor_job(id: u32, conn: &mut Connection<'_, Prover>) {
    conn.compressor_jobs_dal().update_status(id, "queued").await;
}