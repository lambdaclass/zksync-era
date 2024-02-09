/**
 * This suite contains tests checking default ERC-20 contract behavior.
 */

import { TestMaster } from '../src/index';
import { Token } from '../src/types';

import * as zksync from 'zksync-web3';
import * as ethers from 'ethers';
import { ETH_ADDRESS } from 'zksync-web3/build/src/utils';
import { shouldChangeTokenBalances } from '../src/modifiers/balance-checker';

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
        const amount = 500;
        const l1ERC20InitialBalance = await alice.getBalanceL1(tokenDetails.l1Address);
        const initialBalanceL2 = await alice.getBalance();
        // First, a withdraw transaction is done on the L2.
        const withdraw = await alice.withdraw({ token: ETH_ADDRESS, amount });
        const withdrawalHash = withdraw.hash;
        await withdraw.waitFinalize();

        // Get receipt of withdraw transaction, and check the gas cost (fee)
        const receipt = await alice.provider.getTransactionReceipt(withdrawalHash);
        const fee = receipt.effectiveGasPrice.mul(receipt.gasUsed);

        const finalBalanceL2 = await alice.getBalance();
        let expected = initialBalanceL2.sub(amount).sub(fee);
        let actual = finalBalanceL2;
        expect(expected).toStrictEqual(actual);

        // Finalize the withdraw and make sure that the ERC20 balance changes as expected.
        const l1ERC20FinalBalance = await shouldChangeTokenBalances(
            tokenDetails.l1Address,
            [{ wallet: alice, change: amount }],
            { l1: true }
        );
        await expect(alice.finalizeWithdrawal(withdrawalHash)).toBeAccepted([l1ERC20FinalBalance]);

        expect(await alice.getBalanceL1(tokenDetails.l1Address)).toEqual(l1ERC20InitialBalance.add(amount));
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
            expect(err.includes('insufficient balance for transfer')).toBe(true);
        }
    });

    afterAll(async () => {
        await testMaster.deinitialize();
    });
});
