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

async fn restart_prover_job(
    round: AggregationRound,
    id: u32,
    conn: &mut Connection<'_, Prover>,
) -> anyhow::Result<()> {
    let mut conn = conn.start_transaction().await?;
    let mut prover_dal = conn.fri_prover_jobs_dal();

    // Closure to simplify early return with a rollback.
    let inner = || -> anyhow::Result<()> {
        let (real_id, l1_batch_number, circuit_id, status) = prover_dal.get_prover_job_metadata_for_restart(round, id).await?;
        match status {
            "in_progress"|"in_gpu_proof" => return Err(anyhow::anyhow!("Job {} in progress", id)),
            _ => (),
        }
        prover_dal.restart_jobs(vec![real_id]).await?;

        restart_prover_jobs_for_circuit_after_round(round, l1_batch_number, circuit_id, &mut conn).await?;
        restart_witness_jobs_for_circuit_after_round(round, l1_batch_number, circuit_id, &mut conn).await?;

        Ok(())
    };

    // Prioritize the inner error over the transaction error.
    // This way we don't miss reports of jobs in progress.
    let result = inner();
    match result {
        Ok(()) => conn.commit().await?,
        Err(e) => conn.rollback().await?,
    }
    result
}

async fn restart_witness_job(round: AggregationRound, id: u32, conn: &mut Connection<'_, Prover>) -> Result<(), anyhow::Error> {
    let mut conn = conn.start_transaction().await?;
    let mut witness_dal = conn.fri_witness_generator_dal();

    match round {
        BasicCircuits => {
            let (real_id, l1_batch_number, circuit_id, status) = witness_dal.get_leaf_witness_generator_job_metadata_for_restart(round, id).await?;
            match status {
                "in_progress"|"in_gpu_proof" => return Err(anyhow::anyhow!("Job {} in progress", id)),
                _ => (),
            }
            witness_dal.restart_leaf_aggregation_jobs(vec![real_id]).await?;
        }
        LeafAggregation => {
            let (real_id, l1_batch_number, circuit_id, status) = witness_dal.get_leaf_witness_generator_job_metadata_for_restart(round, id).await?;
            match status {
                "in_progress"|"in_gpu_proof" => return Err(anyhow::anyhow!("Job {} in progress", id)),
                _ => (),
            }
            witness_dal.restart_leaf_aggregation_jobs(vec![real_id]).await?;
        }
        NodeAggregation => {
            let (real_id, l1_batch_number, circuit_id, status) = witness_dal.get_node_witness_generator_job_metadata_for_restart(round, id).await?;
            match status {
                "in_progress"|"in_gpu_proof" => return Err(anyhow::anyhow!("Job {} in progress", id)),
                _ => (),
            }
            witness_dal.restart_node_aggregation_jobs(vec![real_id]).await?;
        }
        RecursionTip => {
            let (real_id, l1_batch_number, circuit_id, status) = witness_dal.get_recursion_tip_witness_generator_job_metadata_for_restart(round, id).await?;
            match status {
                "in_progress"|"in_gpu_proof" => return Err(anyhow::anyhow!("Job {} in progress", id)),
                _ => (),
            }
            witness_dal.restart_recursion_tip_jobs(vec![real_id]).await?;
        }
        Scheduler => {
            let (real_id, l1_batch_number, circuit_id, status) = witness_dal.get_scheduler_witness_generator_job_metadata_for_restart(round, id).await?;
            match status {
                "in_progress"|"in_gpu_proof" => return Err(anyhow::anyhow!("Job {} in progress", id)),
                _ => (),
            }
            witness_dal.restart_scheduler_jobs(vec![real_id]).await?;
        }
    }

    let (real_id, l1_batch_number, circuit_id, status) = witness_dal.get_witness_job_metadata_for_restart(round, id).await?;
    match status {
        "in_progress"|"in_gpu_proof" => return Err(anyhow::anyhow!("Job {} in progress", id)),
        _ => (),
    }

    let job_stats = witness_dal.get_witness_jobs_stats_for_batch(l1_batch_number, round).await?;
    let to_restart: Vec<_> = job_stats.iter()
        .filter(|info| info.id == real_id || (info.circuit_id == circuit_id && info.aggregation_round as u8 > round as u8))
        .collect();

    if to_restart.iter().any(|info| matches!(info.status, "in_progress"|"in_gpu_proof")) {
        return Err(anyhow::anyhow!("Some jobs are in progress"));
    }

    let to_restart: Vec<_> = to_restart.iter().map(|info| info.id).collect();
    witness_dal.restart_jobs(to_restart).await?;

    restart_prover_jobs_for_circuit_in_round(round, l1_batch_number, circuit_id, &mut conn).await?;
    restart_prover_jobs_for_circuit_after_round(round, l1_batch_number, circuit_id, &mut conn).await?;
    restart_witness_jobs_for_circuit_after_round(round, l1_batch_number, circuit_id, &mut conn).await?;

    conn.commit().await?;

    Ok(())
}

async fn restart_compressor_job(id: u32, conn: &mut Connection<'_, Prover>) -> Result<(), anyhow::Error> {
    batch::restart_compressor(L1BatchNumber::from(id), conn).await
}

async fn restart_prover_jobs_for_circuit_in_round(
    round: AggregationRound,
    l1_batch_number: L1BatchNumber,
    circuit_id: u32,
    conn: &mut Connection<'_, Prover>,
) -> Result<(), anyhow::Error> {
    let job_stats = prover_dal.get_prover_jobs_stats_for_batch(l1_batch_number, round).await?;
    let to_restart: Vec<_> = job_stats.iter()
        .filter(|info| (info.circuit_id, info.aggregation_round) == (circuit_id, round))
        .collect();

    if to_restart.iter().any(|info| matches!(info.status, "in_progress"|"in_gpu_proof")) {
        return Err(anyhow::anyhow!("Some jobs are in progress"));
    }

    let to_restart: Vec<_> = to_restart.iter().map(|info| info.id).collect();
    prover_dal.restart_jobs(to_restart).await?;

    Ok(())
}

async fn restart_prover_jobs_for_circuit_after_round(
    round: AggregationRound,
    l1_batch_number: L1BatchNumber,
    circuit_id: u32,
    conn: &mut Connection<'_, Prover>,
) -> Result<(), anyhow::Error> {
    let job_stats = prover_dal.get_prover_jobs_stats_for_batch(l1_batch_number, round).await?;
    let to_restart: Vec<_> = job_stats.iter()
        .filter(|info| info.circuit_id == circuit_id && info.aggregation_round as u8 > round as u8)
        .collect();

    if to_restart.iter().any(|info| matches!(info.status, "in_progress"|"in_gpu_proof")) {
        return Err(anyhow::anyhow!("Some jobs are in progress"));
    }

    let to_restart: Vec<_> = to_restart.iter().map(|info| info.id).collect();
    prover_dal.restart_jobs(to_restart).await?;

    Ok(())
}

async fn restart_witness_jobs_for_circuit_after_round(
    round: AggregationRound,
    l1_batch_number: L1BatchNumber,
    circuit_id: u32,
    conn: &mut Connection<'_, Prover>,
) -> Result<(), anyhow::Error> {
    let mut witness_dal = conn.fri_witness_generator_dal();
    let mut next_round = round.next();
    loop {
        match next_round {
            Some(BasicCircuits) => unreachable!("BasicCircuits is the first round"),
            Some(LeafAggregation) => {
                let job_stats = witness_dal.get_leaf_witness_generator_jobs_for_batch(l1_batch_number).await?;
                let to_restart: Vec<_> = job_stats.iter()
                    .filter(|info| info.circuit_id == circuit_id)
                    .collect();
                let to_restart: Vec<_> = to_restart.iter().map(|info| info.id).collect();
                if to_restart.iter().any(|info| matches!(info.status, "in_progress"|"in_gpu_proof")) {
                    return Err(anyhow::anyhow!("Some jobs are in progress"));
                }
                witness_dal.restart_leaf_aggregation_jobs(to_restart).await?;
            }
            Some(NodeAggregation) => {
                let job_stats = witness_dal.get_node_witness_generator_jobs_for_batch(l1_batch_number).await?;
                let to_restart: Vec<_> = job_stats.iter()
                    .filter(|info| info.circuit_id == circuit_id)
                    .collect();
                let to_restart: Vec<_> = to_restart.iter().map(|info| info.id).collect();
                if to_restart.iter().any(|info| matches!(info.status, "in_progress"|"in_gpu_proof")) {
                    return Err(anyhow::anyhow!("Some jobs are in progress"));
                }
                witness_dal.restart_node_aggregation_jobs(to_restart).await?;
            }
            Some(RecursionTip) => {
                let job_stats = witness_dal.get_recursion_tip_witness_generator_jobs_for_batch(l1_batch_number).await?;
                let to_restart: Vec<_> = to_restart.iter().map(|info| info.id).collect();
                if to_restart.iter().any(|info| matches!(info.status, "in_progress"|"in_gpu_proof")) {
                    return Err(anyhow::anyhow!("Some jobs are in progress"));
                }
                witness_dal.restart_recursion_tip_jobs(to_restart).await?;
            }
            Some(Scheduler) => {
                let job_stats = witness_dal.get_scheduler_witness_generator_jobs_for_batch(l1_batch_number).await?;
                let to_restart: Vec<_> = to_restart.iter().map(|info| info.id).collect();
                if to_restart.iter().any(|info| matches!(info.status, "in_progress"|"in_gpu_proof")) {
                    return Err(anyhow::anyhow!("Some jobs are in progress"));
                }
                witness_dal.restart_scheduler_jobs(to_restart).await?;
            }
            None => break,
        }
        next_round = next_round.next();
    }

    Ok(())
}

#[cfg(test)]
mod test {

}