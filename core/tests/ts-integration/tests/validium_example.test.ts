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
import { Provider } from 'zksync-web3';
import { RetryProvider } from '../src/retry-provider';

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const contracts = {
    erc20: getTestContract('ERC20Validium'),
};

describe('Smart contract behavior checks', () => {
    let testMaster: TestMaster;
    let alice: zksync.Wallet;
    let bob: zksync.Wallet;

    beforeAll(() => {
        testMaster = TestMaster.getInstance(__filename);
        alice = testMaster.mainAccount();
        bob = testMaster.newEmptyAccount();
    });

    test('Should deploy validim-example contract', async () => {
        const validimExampleContract = await deployContract(alice, contracts.erc20, ["LambdaToken", "lmd"]);
        await expect(validimExampleContract.name()).resolves.toBe('LambdaToken');

        const response = await validimExampleContract.deployed();
        const deployHash = response.deployTransaction.hash;

        const deployRecipt = await alice.provider.getTransactionReceipt(deployHash);
        console.log("Deploy: ", parseInt(deployRecipt.gasUsed._hex));

        let AliceBlance = await validimExampleContract.balanceOf(alice.address);
        console.log("Alice Balance: ", parseInt(AliceBlance._hex))
        let BobBalance = await validimExampleContract.balanceOf(bob.address);
        console.log("Bob Balance: ", parseInt(BobBalance._hex))

        const mint = await validimExampleContract._mint(alice.address,100000);

        await sleep(5000);

        const mintRecipt = await alice.provider.getTransactionReceipt(mint.hash);
        console.log("Mint: ", parseInt(mintRecipt.gasUsed._hex));

        AliceBlance = await validimExampleContract.balanceOf(alice.address);
        console.log("Alice Balance: ", parseInt(AliceBlance._hex))
        BobBalance = await validimExampleContract.balanceOf(bob.address);
        console.log("Bob Balance: ", parseInt(BobBalance._hex))


        const trasnfer = await validimExampleContract.transfer(bob.address,1000);

        await sleep(5000);

        const transferRecipt = await alice.provider.getTransactionReceipt(trasnfer.hash);
        console.log("Transfer: ", parseInt(transferRecipt.gasUsed._hex));

        AliceBlance = await validimExampleContract.balanceOf(alice.address);
        console.log("Alice Balance: ", parseInt(AliceBlance._hex))
        BobBalance = await validimExampleContract.balanceOf(bob.address);
        console.log("Bob Balance: ", parseInt(BobBalance._hex))
    })

    afterAll(async () => {
        await testMaster.deinitialize();
    });
});
