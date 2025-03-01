use std::path::PathBuf;

use anyhow::Context;
use xshell::{cmd, Shell};
use zkstack_cli_common::{cmd::Cmd, config::global_config, logger};
use zkstack_cli_config::EcosystemConfig;

use super::{
    args::integration::IntegrationArgs,
    utils::{
        build_contracts, install_and_build_dependencies, TestWallets, TEST_WALLETS_PATH,
        TS_INTEGRATION_PATH,
    },
};
use crate::commands::dev::messages::{
    msg_integration_tests_run, MSG_CHAIN_NOT_FOUND_ERR, MSG_DESERIALIZE_TEST_WALLETS_ERR,
    MSG_INTEGRATION_TESTS_RUN_SUCCESS,
};

pub async fn run(shell: &Shell, args: IntegrationArgs) -> anyhow::Result<()> {
    let ecosystem_config = EcosystemConfig::from_file(shell)?;

    let chain_config = ecosystem_config
        .load_current_chain()
        .context(MSG_CHAIN_NOT_FOUND_ERR)?;
    shell.change_dir(ecosystem_config.link_to_code.join(TS_INTEGRATION_PATH));

    logger::info(msg_integration_tests_run(args.external_node));

    if !args.no_deps {
        install_and_build_dependencies(shell, &ecosystem_config)?;
        build_contracts(shell, &ecosystem_config)?;
    }

    let wallets_path: PathBuf = ecosystem_config.link_to_code.join(TEST_WALLETS_PATH);
    let wallets: TestWallets = serde_json::from_str(shell.read_file(&wallets_path)?.as_ref())
        .context(MSG_DESERIALIZE_TEST_WALLETS_ERR)?;

    wallets
        .init_test_wallet(&ecosystem_config, &chain_config)
        .await?;

    let test_pattern = args.test_pattern;
    let mut command = cmd!(
        shell,
        "yarn jest --forceExit --testTimeout 350000 -t {test_pattern...}"
    )
    .env("CHAIN_NAME", ecosystem_config.current_chain())
    .env("MASTER_WALLET_PK", wallets.get_test_pk(&chain_config)?);

    if args.external_node {
        command = command.env("EXTERNAL_NODE", format!("{:?}", args.external_node))
    }

    if global_config().verbose {
        command = command.env(
            "ZKSYNC_DEBUG_LOGS",
            format!("{:?}", global_config().verbose),
        )
    }

    Cmd::new(command).with_force_run().run()?;

    logger::outro(MSG_INTEGRATION_TESTS_RUN_SUCCESS);

    Ok(())
}
