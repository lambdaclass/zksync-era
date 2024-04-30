import { Command } from 'commander';
import { spawn } from 'zk/build/utils';
import fs from 'fs';
import { ethers } from 'ethers';

import { updateContractsEnv } from 'zk/build/contract';
import { getPostUpgradeCalldataFileName } from './utils';

async function hyperchainUpgrade1() {
    const cwd = process.cwd();
    process.chdir(`${process.env.ZKSYNC_HOME}/contracts/l1-contracts/`);

    await spawn(`yarn hyperchain-upgrade-1 | tee deployL1.log`);
    process.chdir(cwd);

    const deployLog = fs.readFileSync(`${process.env.ZKSYNC_HOME}/contracts/l1-contracts/deployL1.log`).toString();
    const l1EnvVars = [
        'CONTRACTS_CREATE2_FACTORY_ADDR',

        'CONTRACTS_BRIDGEHUB_PROXY_ADDR',
        'CONTRACTS_BRIDGEHUB_IMPL_ADDR',

        'CONTRACTS_STATE_TRANSITION_PROXY_ADDR',
        'CONTRACTS_STATE_TRANSITION_IMPL_ADDR',

        'CONTRACTS_ADMIN_FACET_ADDR',
        'CONTRACTS_DIAMOND_UPGRADE_INIT_ADDR',
        'CONTRACTS_DIAMOND_INIT_ADDR',
        'CONTRACTS_DEFAULT_UPGRADE_ADDR',
        'CONTRACTS_HYPERCHAIN_UPGRADE_ADDR',
        'CONTRACTS_GENESIS_UPGRADE_ADDR',
        'CONTRACTS_GOVERNANCE_ADDR',
        'CONTRACTS_MAILBOX_FACET_ADDR',
        'CONTRACTS_EXECUTOR_FACET_ADDR',
        'CONTRACTS_GETTERS_FACET_ADDR',

        'CONTRACTS_VERIFIER_ADDR',
        'CONTRACTS_VALIDATOR_TIMELOCK_ADDR',

        'CONTRACTS_GENESIS_TX_HASH',
        'CONTRACTS_TRANSPARENT_PROXY_ADMIN_ADDR',
        'CONTRACTS_L1_SHARED_BRIDGE_PROXY_ADDR',
        'CONTRACTS_L1_SHARED_BRIDGE_IMPL_ADDR',
        'CONTRACTS_L1_ERC20_BRIDGE_PROXY_ADDR',
        'CONTRACTS_L1_ERC20_BRIDGE_IMPL_ADDR',
        'CONTRACTS_L1_WETH_BRIDGE_IMPL_ADDR',
        'CONTRACTS_L1_WETH_BRIDGE_PROXY_ADDR',
        'CONTRACTS_L1_ALLOW_LIST_ADDR',
        'CONTRACTS_L1_MULTICALL3_ADDR',
        'CONTRACTS_BLOB_VERSIONED_HASH_RETRIEVER_ADDR'
    ];

    console.log('Writing to', `etc/env/l1-inits/${process.env.L1_ENV_NAME ? process.env.L1_ENV_NAME : '.init'}.env`);
    updateContractsEnv(
        `etc/env/l1-inits/${process.env.L1_ENV_NAME ? process.env.L1_ENV_NAME : '.init'}.env`,
        deployLog,
        l1EnvVars
    );
}

async function hyperchainUpgrade2() {
    const cwd = process.cwd();
    process.chdir(`${process.env.ZKSYNC_HOME}/contracts/l1-contracts/`);

    await spawn(`yarn hyperchain-upgrade-2 | tee deployL1.log`);
    process.chdir(cwd);
}

async function preparePostUpgradeCalldata() {
    let calldata = new ethers.utils.AbiCoder().encode(
        ['uint256', 'address', 'address', 'address'],
        [
            process.env.CONTRACTS_ERA_CHAIN_ID,
            process.env.CONTRACTS_BRIDGEHUB_PROXY_ADDR,
            process.env.CONTRACTS_STATE_TRANSITION_PROXY_ADDR,
            process.env.CONTRACTS_L1_SHARED_BRIDGE_PROXY_ADDR
        ]
    );
    let postUpgradeCalldataFileName = getPostUpgradeCalldataFileName(undefined);

    fs.writeFileSync(postUpgradeCalldataFileName, JSON.stringify(calldata, null, 2));
}

export const command = new Command('hyperchain-upgrade').description('create and publish custom l2 upgrade');

command
    .command('start')
    .description('start')
    .option('--phase1')
    .option('--post-upgrade-calldata')
    .option('--phase2')
    .action(async (options) => {
        if (options.phase1) {
            await hyperchainUpgrade1();
        } else if (options.phase2) {
            await hyperchainUpgrade2();
        } else if (options.postUpgradeCalldata) {
            await preparePostUpgradeCalldata();
        }
    });
