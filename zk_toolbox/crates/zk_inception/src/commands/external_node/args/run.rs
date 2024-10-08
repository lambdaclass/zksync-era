use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::messages::{
    MSG_ENABLE_CONSENSUS_HELP, MSG_SERVER_ADDITIONAL_ARGS_HELP, MSG_SERVER_COMPONENTS_HELP,
};

#[derive(Debug, Serialize, Deserialize, Parser)]
pub struct RunExternalNodeArgs {
    #[clap(long)]
    pub reinit: bool,
    #[clap(long, help = MSG_SERVER_COMPONENTS_HELP)]
    pub components: Option<Vec<String>>,
    #[clap(long, help = MSG_ENABLE_CONSENSUS_HELP, default_missing_value = "true", num_args = 0..=1)]
    pub enable_consensus: Option<bool>,
    #[clap(long, short)]
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = false, help = MSG_SERVER_ADDITIONAL_ARGS_HELP)]
    pub additional_args: Vec<String>,
}
