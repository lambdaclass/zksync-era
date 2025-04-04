use std::path::PathBuf;

use clap::Parser;
use xshell::{cmd, Shell};
use zkstack_cli_common::{cmd::Cmd, logger, spinner::Spinner};
use zkstack_cli_config::EcosystemConfig;

use super::sql_fmt::format_sql;
use crate::commands::dev::{
    commands::lint_utils::{get_unignored_files, Target},
    messages::{
        msg_running_fmt_for_extension_spinner, msg_running_fmt_for_extensions_spinner,
        msg_running_rustfmt_for_dir_spinner, MSG_RUNNING_CONTRACTS_FMT_SPINNER,
    },
};

async fn prettier(shell: Shell, target: Target, check: bool) -> anyhow::Result<()> {
    let spinner = Spinner::new(&msg_running_fmt_for_extension_spinner(target));
    let files = get_unignored_files(&shell, &target, None)?;

    if files.is_empty() {
        logger::info(format!("No files for {target} found"));
        return Ok(());
    }

    spinner.freeze();
    let mode = if check { "--check" } else { "--write" };
    let config = format!("etc/prettier-config/{target}.js");
    Ok(
        Cmd::new(cmd!(shell, "yarn --silent prettier {mode} --config {config}").args(files))
            .run()?,
    )
}

async fn prettier_contracts(shell: Shell, check: bool) -> anyhow::Result<()> {
    let spinner = Spinner::new(MSG_RUNNING_CONTRACTS_FMT_SPINNER);
    spinner.freeze();
    let prettier_command = cmd!(shell, "yarn --silent --cwd contracts")
        .arg(format!("prettier:{}", if check { "check" } else { "fix" }));

    Ok(Cmd::new(prettier_command).run()?)
}

async fn rustfmt(shell: Shell, check: bool, link_to_code: PathBuf) -> anyhow::Result<()> {
    for dir in ["core", "prover", "zkstack_cli"] {
        let spinner = Spinner::new(&msg_running_rustfmt_for_dir_spinner(dir));
        let _dir = shell.push_dir(link_to_code.join(dir));
        let mut cmd = cmd!(shell, "cargo fmt -- --config imports_granularity=Crate --config group_imports=StdExternalCrate");
        if check {
            cmd = cmd.arg("--check");
        }
        spinner.freeze();
        Cmd::new(cmd).run()?;
    }
    Ok(())
}

async fn run_all_rust_formatters(
    shell: Shell,
    check: bool,
    link_to_code: PathBuf,
) -> anyhow::Result<()> {
    format_sql(shell.clone(), check).await?;
    rustfmt(shell.clone(), check, link_to_code).await?;
    Ok(())
}

#[derive(Debug, Parser)]
pub enum Formatter {
    Rustfmt,
    Contract,
    Prettier {
        #[arg(short, long)]
        targets: Vec<Target>,
    },
}

#[derive(Debug, Parser)]
pub struct FmtArgs {
    #[clap(long, short = 'c')]
    pub check: bool,
    #[clap(subcommand)]
    pub formatter: Option<Formatter>,
}

pub async fn run(shell: Shell, args: FmtArgs) -> anyhow::Result<()> {
    let ecosystem = EcosystemConfig::from_file(&shell)?;
    shell.set_var("ZKSYNC_USE_CUDA_STUBS", "true");
    match args.formatter {
        None => {
            let mut tasks = vec![];
            let extensions: Vec<_> = vec![Target::Js, Target::Ts, Target::Md, Target::Sol];
            let spinner = Spinner::new(&msg_running_fmt_for_extensions_spinner(&extensions));
            spinner.freeze();
            for ext in extensions {
                tasks.push(tokio::spawn(prettier(shell.clone(), ext, args.check)));
            }
            tasks.push(tokio::spawn(run_all_rust_formatters(
                shell.clone(),
                args.check,
                ecosystem.link_to_code,
            )));
            tasks.push(tokio::spawn(prettier_contracts(shell.clone(), args.check)));

            for result in futures::future::join_all(tasks).await {
                result??;
            }
        }
        Some(Formatter::Prettier { mut targets }) => {
            if targets.is_empty() {
                targets = vec![Target::Js, Target::Ts, Target::Md, Target::Sol];
            }
            let spinner = Spinner::new(&msg_running_fmt_for_extensions_spinner(&targets));
            for target in targets {
                prettier(shell.clone(), target, args.check).await?
            }
            spinner.finish()
        }
        Some(Formatter::Rustfmt) => {
            run_all_rust_formatters(shell.clone(), args.check, ".".into()).await?
        }
        Some(Formatter::Contract) => prettier_contracts(shell.clone(), args.check).await?,
    }
    Ok(())
}
