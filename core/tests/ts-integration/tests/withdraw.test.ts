/**
 * This suite contains tests checking default ERC-20 contract behavior.
 */

import { TestMaster } from '../src/index';
import { Token } from '../src/types';
import { shouldChangeTokenBalances, shouldOnlyTakeFee } from '../src/modifiers/balance-checker';

import * as zksync from 'zksync-web3';
import { BigNumber, utils as etherUtils } from 'ethers';
import * as ethers from 'ethers';
import { scaledGasPrice, waitUntilBlockFinalized } from '../src/helpers';
import { L2_ETH_PER_ACCOUNT } from '../src/context-owner';
import { ETH_ADDRESS } from 'zksync-web3/build/src/utils';
import { sleep } from 'zk/build/utils';

describe('ERC20 contract checks', () => {
    let testMaster: TestMaster;
    let alice: zksync.Wallet;
    let tokenDetails: Token;
    let aliceErc20: ethers.Contract;
    const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

    beforeAll(async () => {
        testMaster = TestMaster.getInstance(__filename);
        alice = testMaster.mainAccount();

        tokenDetails = testMaster.environment().erc20Token;
        aliceErc20 = (await alice.getL1BridgeContracts()).erc20;
    });

    test('Can perform a valid withdrawal', async () => {
        // TODO: make sure that the test is not run in the fast mode
        if (testMaster.isFastMode()) {
            return;
        }
        const amount = 1;

        const initialBalanceL2 = await alice.getBalance();
        const initialBalanceL1 = await alice.getBalanceL1(tokenDetails.l1Address);

        // First, a withdraw transaction is done on the L2,
        const withdraw = await alice.withdraw({ token: ETH_ADDRESS, amount });
        const withdrawalHash = withdraw.hash;
        withdraw.waitFinalize();

        // sleep for 10 seconds to wait for the transaction to be mined
        await sleep(10000);

        // Balance should be 1gwei + Xgwei (fee) less than the initial balance
        // TODO: check if there is a way to get the specific fee value.
        const finalBalanceL2 = await alice.getBalance();
        let expected = initialBalanceL2.sub(amount).toString();
        let actual = finalBalanceL2.toString();
        expect(actual <= expected);

        // Afterwards, a withdraw-finalize is done on the L1,
        await sleep(1000);
        (await alice.finalizeWithdrawal(withdrawalHash)).wait();

        // make sure that the balance on the L1 has increased by the amount withdrawn
        await sleep(1000);
        const finalBalanceL1 = await alice.getBalanceL1(tokenDetails.l1Address);
        expected = initialBalanceL1.add(amount).toString();
        actual = finalBalanceL1.toString();
        expect(actual == expected);
    });

    test(`Can't perform an invalid withdrawal`, async () => {
        // TODO: make sure that the test is not run in the fast mode
        if (testMaster.isFastMode()) {
            return;
        }

        const initialBalanceL2 = await alice.getBalance();
        const amount = initialBalanceL2.add(1);
        try {
            await alice.withdraw({ token: ETH_ADDRESS, amount });
        } catch (e: any) {
            const err = e.toString();
            expect(err.includes('insufficient balance for transfer'));
        }
    });

    afterAll(async () => {
        await testMaster.deinitialize();
    });
});
