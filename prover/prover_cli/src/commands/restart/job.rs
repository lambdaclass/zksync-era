use clap::Args as ClapArgs;
use prover_dal::{Connection, ConnectionPool, Prover, ProverDal};

use crate::cli::ProverCLIConfig;

#[derive(ClapArgs)]
pub struct Args {
    id: u32,
}

pub async fn run(args: Args, config: ProverCLIConfig) -> anyhow::Result<()> {
    let connection_pool =
        //FIXME master_url?
        ConnectionPool::<Prover>::singleton(config.db_url)
            .build()
            .await?;
    let mut conn = connection_pool.connection().await?;
    restart_prover_job(args.id, &mut conn).await;
    Ok(())
}

async fn restart_prover_job(id: u32, conn: &mut Connection<'_, Prover>) {
    conn.fri_prover_jobs_dal().update_status(id, "queued").await;
}
