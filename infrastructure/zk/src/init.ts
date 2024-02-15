import chalk from 'chalk';
import { Command } from 'commander';
import * as utils from './utils';

import { clean } from './clean';
import * as compiler from './compiler';
import * as contract from './contract';
import * as db from './database';
import * as docker from './docker';
import * as env from './env';
import * as run from './run/run';
import * as server from './server';
import { up } from './up';

import * as fs from 'fs';

const entry = chalk.bold.yellow;
const announce = chalk.yellow;
const success = chalk.green;
const timestamp = chalk.grey;

export async function init(initArgs: InitArgs = DEFAULT_ARGS) {
    const {
        skipSubmodulesCheckout,
        skipEnvSetup,
        testTokens,
        governorPrivateKeyArgs,
        deployerL2ContractInput,
        validiumMode
    } = initArgs;

    configMode(validiumMode);

    if (!process.env.CI && !skipEnvSetup) {
        await announced('Pulling images', docker.pull());
        await announced('Checking environment', checkEnv());
        await announced('Checking git hooks', env.gitHooks());
        await announced('Setting up containers', up());
    }
    if (!skipSubmodulesCheckout) {
        await announced('Checkout system-contracts submodule', submoduleUpdate());
    }

    await announced('Compiling JS packages', run.yarn());
    await announced('Compile l2 contracts', compiler.compileAll());
    await announced('Drop postgres db', db.drop());
    await announced('Setup postgres db', db.setup());
    await announced('Clean rocksdb', clean('db'));
    await announced('Clean backups', clean('backups'));
    await announced('Building contracts', contract.build());
    if (testTokens.deploy) {
        await announced('Deploying localhost ERC20 tokens', run.deployERC20('dev', '', '', '', testTokens.args));
    }
    await announced('Deploying L1 verifier', contract.deployVerifier([]));
    await announced('Reloading env', env.reload());
    await announced('Running server genesis setup', server.genesisFromSources());
    await announced('Deploying L1 contracts', contract.redeployL1(governorPrivateKeyArgs));
    await announced('Initializing validator', contract.initializeValidator(governorPrivateKeyArgs));
    await announced(
        'Deploying L2 contracts',
        contract.deployL2(
            deployerL2ContractInput.args,
            deployerL2ContractInput.includePaymaster,
            deployerL2ContractInput.includeL2WETH
        )
    );

    if (deployerL2ContractInput.includeL2WETH) {
        await announced('Initializing L2 WETH token', contract.initializeWethToken(governorPrivateKeyArgs));
    }
    await announced('Initializing governance', contract.initializeGovernance(governorPrivateKeyArgs));
}

// A smaller version of `init` that "resets" the localhost environment, for which `init` was already called before.
// It does less and runs much faster.
export async function reinit(validiumMode: boolean) {
    process.env.VALIDIUM_MODE = validiumMode.toString();
    await announced(`Initializing in ${validiumMode ? 'Validium mode' : 'Roll-up mode'}`);

    await announced('Setting up containers', up());
    await announced('Compiling JS packages', run.yarn());
    await announced('Compile l2 contracts', compiler.compileAll());
    await announced('Drop postgres db', db.drop());
    await announced('Setup postgres db', db.setup());
    await announced('Clean rocksdb', clean('db'));
    await announced('Clean backups', clean('backups'));
    await announced('Building contracts', contract.build());
    await announced('Deploying L1 verifier', contract.deployVerifier([]));
    await announced('Reloading env', env.reload());
    await announced('Running server genesis setup', server.genesisFromSources());
    await announced('Deploying L1 contracts', contract.redeployL1([]));
    await announced('Deploying L2 contracts', contract.deployL2([], true, true));
    await announced('Initializing L2 WETH token', contract.initializeWethToken());
    await announced('Initializing governance', contract.initializeGovernance());
    await announced('Initializing validator', contract.initializeValidator());
}

// A lightweight version of `init` that sets up local databases, generates genesis and deploys precompiled contracts
export async function lightweightInit(validiumMode: boolean) {
    process.env.VALIDIUM_MODE = validiumMode.toString();
    await announced(`Initializing in ${validiumMode ? 'Validium mode' : 'Roll-up mode'}`);

    await announced(`Setting up containers`, up());
    await announced('Clean rocksdb', clean('db'));
    await announced('Clean backups', clean('backups'));
    await announced('Deploying L1 verifier', contract.deployVerifier([]));
    await announced('Reloading env', env.reload());
    await announced('Running server genesis setup', server.genesisFromBinary());
    await announced('Deploying localhost ERC20 tokens', run.deployERC20('dev', '', '', '', []));
    await announced('Deploying L1 contracts', contract.redeployL1([]));
    await announced('Initializing validator', contract.initializeValidator());
    await announced('Deploying L2 contracts', contract.deployL2([], true, false));
    await announced('Initializing governance', contract.initializeGovernance());
}

// Wrapper that writes an announcement and completion notes for each executed task.
export async function announced(fn: string, promise: Promise<void> | void) {
    const announceLine = `${entry('>')} ${announce(fn)}`;
    const separator = '-'.repeat(fn.length + 2); // 2 is the length of "> ".
    console.log(`\n` + separator); // So it's easier to see each individual step in the console.
    console.log(announceLine);

    const start = new Date().getTime();
    // The actual execution part
    await promise;

    const time = new Date().getTime() - start;
    const successLine = `${success('✔')} ${fn} done`;
    const timestampLine = timestamp(`(${time}ms)`);
    console.log(`${successLine} ${timestampLine}`);
}

export async function submoduleUpdate() {
    await utils.exec('git submodule init');
    await utils.exec('git submodule update');
}

async function configMode(validiumMode: boolean) {
    let envFileContent = fs.readFileSync(process.env.ENV_FILE!).toString();
    envFileContent += `VALIDIUM_MODE=${validiumMode}\n`;
    fs.writeFileSync(process.env.ENV_FILE!, envFileContent);
    await announced(`Initializing in ${validiumMode ? 'Validium mode' : 'Roll-up mode'}`);

    const chainPath = 'etc/env/base/chain.toml';
    const modeConstantValues = {
        pubdata_overhead_part: validiumMode ? 0.0 : 1.0,
        batch_overhead_l1_gas: validiumMode ? 1000000 : 800000,
        max_pubdata_per_batch: validiumMode ? 1000000000000 : 100000
    };

    let chainContent = fs.readFileSync(chainPath, 'utf-8');
    const lines = chainContent.split('\n');

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        for (const [key, value] of Object.entries(modeConstantValues)) {
            if (line.includes(`${key}=`)) {
                lines[i] = `${key}=${value}`;
                break;
            }
        }
    }
    chainContent = lines.join('\n');
    fs.writeFileSync(chainPath, chainContent);

    await announced(`The parameters have been updated in the ${chainPath} file.`);

    const ethSenderPath = 'etc/env/base/eth_sender.toml';
    const enforcedGasPrice = 'internal_enforced_l1_gas_price';
    const newValue = 45_000_000_000;

    let ethSenderToml = fs.readFileSync(ethSenderPath, 'utf-8');

    const linesEthSender = ethSenderToml.split('\n');
    let found = false;

    for (let i = 0; i < linesEthSender.length; i++) {
        const line = linesEthSender[i];
        if (line.includes(`${enforcedGasPrice}=`)) {
            linesEthSender[i] = validiumMode ? `${enforcedGasPrice}=${newValue}` : '';
            found = true;
            break;
        }
    }

    if (!found) {
        if (validiumMode) {
            linesEthSender.push(`${enforcedGasPrice}=${newValue}`);
        }
    }

    ethSenderToml = linesEthSender.join('\n');

    fs.writeFileSync(ethSenderPath, ethSenderToml);

    await announced(`The parameter "${enforcedGasPrice}" has been updated in the TOML file.\n`);
}

async function checkEnv() {
    const tools = ['node', 'yarn', 'docker', 'cargo'];
    for (const tool of tools) {
        await utils.exec(`which ${tool}`);
    }
    const { stdout: version } = await utils.exec('node --version');
    // Node v14.14 is required because
    // the `fs.rmSync` function was added in v14.14.0
    if ('v14.14' >= version) {
        throw new Error('Error, node.js version 14.14.0 or higher is required');
    }
}

export interface InitArgs {
    skipSubmodulesCheckout: boolean;
    skipEnvSetup: boolean;
    governorPrivateKeyArgs: any[];
    deployerL2ContractInput: {
        args: any[];
        includePaymaster: boolean;
        includeL2WETH: boolean;
    };
    testTokens: {
        deploy: boolean;
        args: any[];
    };
    validiumMode: boolean;
}

const DEFAULT_ARGS: InitArgs = {
    skipSubmodulesCheckout: false,
    skipEnvSetup: false,
    governorPrivateKeyArgs: [],
    deployerL2ContractInput: { args: [], includePaymaster: true, includeL2WETH: true },
    testTokens: { deploy: true, args: [] },
    validiumMode: false
};

export const initCommand = new Command('init')
    .option('--skip-submodules-checkout')
    .option('--skip-env-setup')
    .option('--validium-mode')
    .description('perform zksync network initialization for development')
    .action(async (cmd: Command) => {
        const initArgs: InitArgs = {
            skipSubmodulesCheckout: cmd.skipSubmodulesCheckout,
            skipEnvSetup: cmd.skipEnvSetup,
            governorPrivateKeyArgs: [],
            deployerL2ContractInput: { args: [], includePaymaster: true, includeL2WETH: true },
            testTokens: { deploy: true, args: [] },
            validiumMode: cmd.validiumMode !== undefined ? cmd.validiumMode : false
        };
        await init(initArgs);
    });
export const reinitCommand = new Command('reinit')
    .description('"reinitializes" network. Runs faster than `init`, but requires `init` to be executed prior')
    .action(async (cmd: Command) => {
        await reinit(cmd.validiumMode);
    });
export const lightweightInitCommand = new Command('lightweight-init')
    .description('perform lightweight zksync network initialization for development')
    .action(async (cmd: Command) => {
        await lightweightInit(cmd.validiumMode);
    });
