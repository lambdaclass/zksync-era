use clap::Args as ClapArgs;
use prover_dal::{Connection, ConnectionPool, Prover, ProverDal};
use zksync_basic_types::{
    basic_fri_types::AggregationRound::{self, *},
    L1BatchNumber,
    prover_dal::{ProverJobStatus, WitnessJobStatus},
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
    let mut tx = conn.start_transaction().await?;
    async fn inner(args: Args, conn: &mut Connection<'_, Prover>) -> anyhow::Result<()> {
        if let Some(round) = args.witness_job {
            return restart_witness_job(round, args.id, conn).await;
        }
        if let Some(round) = args.prover_job {
            return restart_prover_job(round, args.id, conn).await;
        }
        if let Some(_) = args.compressor_job {
            return restart_compressor_job(args.id, conn).await;
        }
        // This case is filtered by the required argument group.
        unreachable!();
    }

    // Prioritize the inner error over the transaction error.
    // This way we don't miss reports of jobs in progress.
    let result = inner(args, &mut tx).await;
    match result {
        Ok(()) => tx.commit().await?,
        Err(_) => tx.rollback().await?,
    }
    result
}

async fn restart_prover_job(
    round: AggregationRound,
    id: u32,
    conn: &mut Connection<'_, Prover>,
) -> anyhow::Result<()> {
    let mut prover_dal = conn.fri_prover_jobs_dal();

    // Closure to simplify early return with a rollback.
    let (real_id, l1_batch_number, circuit_id, status) = prover_dal.get_prover_job_metadata_for_restart(round, id).await?;
    match status.as_ref() {
        "in_progress"|"in_gpu_proof" => return Err(anyhow::anyhow!("Job {} in progress", id)),
        _ => (),
    }
    prover_dal.restart_jobs(&vec![real_id as i64]).await?;

    restart_prover_jobs_for_circuit_after_round(round, l1_batch_number, circuit_id, conn).await?;
    restart_witness_jobs_for_circuit_after_round(round, l1_batch_number, circuit_id, conn).await?;
    batch::restart_compressor(l1_batch_number, conn).await?;

    Ok(())
}

async fn restart_witness_job(round: AggregationRound, id: u32, conn: &mut Connection<'_, Prover>) -> Result<(), anyhow::Error> {
    let mut dal = conn.fri_witness_generator_dal();

    if matches!(round, BasicCircuits|RecursionTip|Scheduler) {
        return batch::restart_from_aggregation_round(round, L1BatchNumber::from(id), conn).await;
    }
    let (l1_batch_number, circuit_id, status) = dal.get_leaf_or_node_witness_generator_job_metadata_for_restart(round, id).await?;
    match status.as_ref() {
        "in_progress"|"in_gpu_proof" => return Err(anyhow::anyhow!("Job {} in progress", id)),
        _ => (),
    }
    match round {
        LeafAggregation => {
            dal.restart_leaf_aggregation_jobs(vec![id as i64]).await?;
        }
        NodeAggregation => {
            dal.restart_node_aggregation_jobs(vec![id as i64]).await?;
        }
        _ => unreachable!("Already handled BasicCircuits, RecursionTip and Scheduler"),
    }

    restart_prover_jobs_for_circuit_in_round(round, l1_batch_number, circuit_id, conn).await?;
    restart_prover_jobs_for_circuit_after_round(round, l1_batch_number, circuit_id, conn).await?;
    restart_witness_jobs_for_circuit_after_round(round, l1_batch_number, circuit_id, conn).await?;
    batch::restart_compressor(l1_batch_number, conn).await?;

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
    let mut dal = conn.fri_prover_jobs_dal();
    let job_stats = dal.get_prover_jobs_stats_for_batch(l1_batch_number, round).await;
    let to_restart: Vec<_> = job_stats.iter()
        .filter(|info| (info.circuit_id, info.aggregation_round) == (circuit_id, round))
        .collect();

    if to_restart.iter().any(|info| matches!(info.status, ProverJobStatus::InProgress(_)|ProverJobStatus::InGPUProof)) {
        return Err(anyhow::anyhow!("Some jobs are in progress"));
    }

    let to_restart: Vec<_> = to_restart.iter().map(|info| info.id as i64).collect();
    dal.restart_jobs(&to_restart).await?;

    Ok(())
}

async fn restart_prover_jobs_for_circuit_after_round(
    round: AggregationRound,
    l1_batch_number: L1BatchNumber,
    circuit_id: u32,
    conn: &mut Connection<'_, Prover>,
) -> Result<(), anyhow::Error> {
    let mut dal = conn.fri_prover_jobs_dal();
    let mut current_round = round.next();
    let mut to_restart = Vec::new();

    loop {
        let Some(round) = current_round else {
            break;
        };
        let job_stats = dal.get_prover_jobs_stats_for_batch(l1_batch_number, round).await;
        let to_restart_in_round: Vec<_> = job_stats.iter()
            .filter(|info| info.circuit_id == circuit_id)
            .collect();
        if to_restart_in_round.iter().any(|info| matches!(info.status, ProverJobStatus::InProgress(_)|ProverJobStatus::InGPUProof)) {
            return Err(anyhow::anyhow!("Some jobs are in progress"));
        }
        to_restart.extend(to_restart_in_round.iter().map(|info| info.id as i64));
        current_round = current_round.and_then(|round| round.next());
    }

    dal.restart_jobs(&to_restart).await?;

    Ok(())
}

async fn restart_witness_jobs_for_circuit_after_round(
    round: AggregationRound,
    l1_batch_number: L1BatchNumber,
    circuit_id: u32,
    conn: &mut Connection<'_, Prover>,
) -> Result<(), anyhow::Error> {
    let mut dal = conn.fri_witness_generator_dal();
    let mut next_round = round.next();
    loop {
        match next_round {
            Some(BasicCircuits) => unreachable!("BasicCircuits is the first round"),
            Some(round @ (RecursionTip|Scheduler)) => {
                return batch::restart_from_aggregation_round(round, l1_batch_number, conn).await;
            }
            Some(LeafAggregation) => {
                let job_stats = dal.get_leaf_witness_generator_jobs_for_batch(l1_batch_number).await;
                let to_restart: Vec<_> = job_stats.iter()
                    .filter(|info| info.circuit_id == circuit_id)
                    .collect();
                if to_restart.iter().any(|info| matches!(info.status, WitnessJobStatus::InProgress)) {
                    return Err(anyhow::anyhow!("Some jobs are in progress"));
                }
                let to_restart: Vec<_> = to_restart.iter().map(|info| info.id as i64).collect();
                dal.restart_leaf_aggregation_jobs(to_restart).await?;
            }
            Some(NodeAggregation) => {
                let job_stats = dal.get_node_witness_generator_jobs_for_batch(l1_batch_number).await;
                let to_restart: Vec<_> = job_stats.iter()
                    .filter(|info| info.circuit_id == circuit_id)
                    .collect();
                if to_restart.iter().any(|info| matches!(info.status, WitnessJobStatus::InProgress)) {
                    return Err(anyhow::anyhow!("Some jobs are in progress"));
                }
                let to_restart: Vec<_> = to_restart.iter().map(|info| info.id as i64).collect();
                dal.restart_node_aggregation_jobs(to_restart).await?;
            }
            None => break,
        }
        next_round = next_round.and_then(|round| round.next());
    }

    Ok(())
}

#[cfg(test)]
mod test {

}