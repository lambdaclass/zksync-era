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
import * as constants from './constants';

const entry = chalk.bold.yellow;
const announce = chalk.yellow;
const success = chalk.green;
const timestamp = chalk.grey;
const CHAIN_CONFIG_PATH = 'etc/env/base/chain.toml';
const ETH_SENDER_PATH = 'etc/env/base/eth_sender.toml';

export async function init(initArgs: InitArgs = DEFAULT_ARGS) {
    const {
        skipSubmodulesCheckout,
        skipEnvSetup,
        testTokens,
        governorPrivateKeyArgs,
        deployerL2ContractInput,
        validiumMode
    } = initArgs;

    await announced(`Initializing in ${validiumMode ? 'Validium mode' : 'Roll-up mode'}`);
    await announced('Updating mode configuration', updateConfig(validiumMode));
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
    await announced(`Initializing in ${validiumMode ? 'Validium mode' : 'Roll-up mode'}`);
    await announced('Updating mode configuration', updateConfig(validiumMode));
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
    await announced(`Initializing in ${validiumMode ? 'Validium mode' : 'Roll-up mode'}`);
    await announced('Updating mode configuration', updateConfig(validiumMode));
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

function updateConfigFile(path: string, modeConstantValues: any) {
    let content = fs.readFileSync(path, 'utf-8');
    let lines = content.split('\n');
    let entries = Object.entries(modeConstantValues);
    let addedContent;
    while (entries.length > 0) {
        const [key, value] = entries.pop()!;
        let lineIndex = lines.findIndex((line) => !line.startsWith('#') && line.includes(`${key}=`));

        if (lineIndex !== -1) {
            if (value !== null) {
                lines.splice(lineIndex, 1, `${key}=${value}`);
            } else {
                lines.splice(lineIndex, 1);
                for (const [k, index] of Object.entries(lineIndices)) {
                    if (index > lineIndex) {
                        lineIndices[k] = index - 1;
                    }
                }
            }
        } else {
            if (value !== null) {
                addedContent = `${key}=${value}\n`;
            }
        }
    }
    content = lines.join('\n');
    if (addedContent) {
        content += addedContent;
    }
    fs.writeFileSync(path, content);
}
function updateChainConfig(validiumMode: boolean) {
    const modeConstantValues = {
        compute_overhead_part: validiumMode
            ? constants.VALIDIUM_COMPUTE_OVERHEAD_PART
            : constants.ROLLUP_COMPUTE_OVERHEAD_PART,
        pubdata_overhead_part: validiumMode
            ? constants.VALIDIUM_PUBDATA_OVERHEAD_PART
            : constants.ROLLUP_PUBDATA_OVERHEAD_PART,
        batch_overhead_l1_gas: validiumMode
            ? constants.VALIDIUM_BATCH_OVERHEAD_L1_GAS
            : constants.ROLLUP_BATCH_OVERHEAD_L1_GAS,
        max_pubdata_per_batch: validiumMode
            ? constants.VALIDIUM_MAX_PUBDATA_PER_BATCH
            : constants.ROLLUP_MAX_PUBDATA_PER_BATCH,
        l1_batch_commit_data_generator_mode: validiumMode
            ? constants.VALIDIUM_L1_BATCH_COMMIT_DATA_GENERATOR_MODE
            : constants.ROLLUP_L1_BATCH_COMMIT_DATA_GENERATOR_MODE
    };
    updateConfigFile(CHAIN_CONFIG_PATH, modeConstantValues);
}
function updateEthSenderConfig(validiumMode: boolean) {
    // This constant is used in validium mode and is deleted in rollup mode
    // In order to pass the existing integration tests
    const modeConstantValues = {
        internal_enforced_l1_gas_price: validiumMode
            ? constants.VALIDIUM_ENFORCED_L1_GAS_PRICE
            : constants.ROLLUP_ENFORCED_L1_GAS_PRICE,
        l1_gas_per_pubdata_byte: validiumMode
            ? constants.VALIDIUM_L1_GAS_PER_PUBDATA_BYTE
            : constants.ROLLUP_L1_GAS_PER_PUBDATA_BYTE
    };
    updateConfigFile(ETH_SENDER_PATH, modeConstantValues);
}

function updateConfig(validiumMode: boolean) {
    updateChainConfig(validiumMode);
    updateEthSenderConfig(validiumMode);
    env.load();
    let envFileContent = fs.readFileSync(process.env.ENV_FILE!).toString();
    envFileContent += `VALIDIUM_MODE=${validiumMode}\n`;
    fs.writeFileSync(process.env.ENV_FILE!, envFileContent);
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
