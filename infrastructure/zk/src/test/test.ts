import chalk from 'chalk';
import { Command } from 'commander';
import * as utils from '../utils';

import * as integration from './integration';
import * as db from '../database';

export { integration };

export async function prover() {
    process.chdir(process.env.ZKSYNC_HOME! + '/prover');
    await utils.spawn('cargo test --release --workspace --locked');
}

export async function rust(options: string[]) {
    await db.resetTest({ core: true, prover: true });

    let result = await utils.exec('cargo install --list');
    let test_runner = 'cargo nextest run';
    if (!result.stdout.includes('cargo-nextest')) {
        console.warn(
            chalk.bold.red(
                `cargo-nextest is missing, please run "cargo install cargo-nextest". Falling back to "cargo test".`
            )
        );
        test_runner = 'cargo test';
    }

    let cmd = `${test_runner} --release ${options.join(' ')}`;
    console.log(`running unit tests with '${cmd}'`);

    await utils.spawn(cmd);
}

export const command = new Command('test').description('run test suites').addCommand(integration.command);

command.command('prover').description('run unit-tests for the prover').action(prover);
command
    .command('rust [command...]')
    .allowUnknownOption()
    .description(
        'run unit-tests. the default is running all tests in all rust bins and libs. accepts optional arbitrary cargo test flags'
    )
    .action(async (args: string[]) => {
        await rust(args);
    });
