use std::path::PathBuf;

use anyhow::Context;
use xshell::Shell;
use zkstack_cli_common::logger;
use zkstack_cli_config::EcosystemConfig;

use super::utils::{TestWallets, TEST_WALLETS_PATH};
use crate::commands::dev::messages::{
    MSG_DESERIALIZE_TEST_WALLETS_ERR, MSG_TEST_WALLETS_INFO, MSG_WALLETS_TEST_SUCCESS,
};

pub fn run(shell: &Shell) -> anyhow::Result<()> {
    logger::info(MSG_TEST_WALLETS_INFO);

    let ecosystem_config = EcosystemConfig::from_file(shell)?;

    let chain_config = ecosystem_config
        .load_current_chain()
        .context("Chain not found")?;

    let wallets_path: PathBuf = ecosystem_config.link_to_code.join(TEST_WALLETS_PATH);
    let wallets: TestWallets = serde_json::from_str(shell.read_file(wallets_path)?.as_ref())
        .context(MSG_DESERIALIZE_TEST_WALLETS_ERR)?;

    logger::info(format!("Main: {:#?}", wallets.get_main_wallet()?));
    logger::info(format!(
        "Chain: {:#?}",
        wallets.get_test_wallet(&chain_config)?
    ));

    logger::outro(MSG_WALLETS_TEST_SUCCESS);

    Ok(())
}
