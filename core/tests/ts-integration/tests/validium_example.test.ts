/**
 * Generic tests checking the deployed smart contract behavior.
 *
 * Note: if you are going to write multiple tests checking specific topic (e.g. `CREATE2` behavior or something like this),
 * consider creating a separate suite.
 * Let's try to keep only relatively simple and self-contained tests here.
 */

import { TestMaster } from '../src/index';
import { deployContract, getTestContract } from '../src/helpers';

import * as ethers from 'ethers';
import * as zksync from 'zksync-web3';
import { shouldChangeTokenBalances, shouldOnlyTakeFee } from '../src/modifiers/balance-checker';

const contracts = {
    erc20: getTestContract('ERC20Validium')
};

describe('Smart contract behavior checks', () => {
    let testMaster: TestMaster;
    let alice: zksync.Wallet;
    let bob: zksync.Wallet;
    let erc20: ethers.Contract

    beforeAll(() => {
        testMaster = TestMaster.getInstance(__filename);
        alice = testMaster.mainAccount();
        bob = testMaster.newEmptyAccount();
        
    });

    let validimExampleContract: zksync.Contract

    test('Deploy', async () => {
        validimExampleContract = await deployContract(alice, contracts.erc20, ['LambdaToken', 'lmd']);
        await expect(validimExampleContract.name()).resolves.toBe('LambdaToken');

        const response = await validimExampleContract.deployed();
        const deployHash = response.deployTransaction.hash;

        const deployRecipt = await alice.provider.getTransactionReceipt(deployHash);
        console.log('Deploy gas: ', parseInt(deployRecipt.gasUsed._hex));
    });

    test('Mint', async () => {

        const value = 10000
        const mint = await validimExampleContract._mint(alice.address, value);
        await mint.waitFinalize();

        const mintRecipt = await alice.provider.getTransactionReceipt(mint.hash);
        console.log('Mint gas: ', parseInt(mintRecipt.gasUsed._hex));

        const balanceChange = await shouldChangeTokenBalances(validimExampleContract.address, [
            { wallet: alice, change: value }
        ]);
    });

    test('Transfer', async () => {
        const value = 1000;
        const trasnfer = await validimExampleContract.transfer(bob.address, value);
        await trasnfer.waitFinalize();

        const transferRecipt = await alice.provider.getTransactionReceipt(trasnfer.hash);
        console.log('Transfer: ', parseInt(transferRecipt.gasUsed._hex));

        const balanceChange = await shouldChangeTokenBalances(validimExampleContract.address, [
            { wallet: alice, change: -value },
            { wallet: bob, change: value }
        ]);
        const feeCheck = await shouldOnlyTakeFee(alice);
    });

    afterAll(async () => {
        await testMaster.deinitialize();
    });
});
