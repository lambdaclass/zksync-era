use clap::Subcommand;
use crate::cli::ProverCLIConfig;

pub(crate) mod batch;
pub(crate) mod job;

#[derive(Subcommand)]
pub enum RestartCommand {
    Batch(batch::Args),
    Job(job::Args),
}

impl RestartCommand {
    pub(crate) async fn run(self, config: ProverCLIConfig) -> anyhow::Result<()> {
        match self {
            RestartCommand::Batch(args) => batch::run(args, config).await,
            RestartCommand::Job(args) => job::run(args, config).await,
        }
    }
}
