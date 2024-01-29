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

describe('ERC20 contract checks', () => {
    let testMaster: TestMaster;
    let alice: zksync.Wallet;
    let bob: zksync.Wallet;
    let tokenDetails: Token;
    let aliceErc20: zksync.Contract;

    beforeAll(async () => {
        testMaster = TestMaster.getInstance(__filename);
        alice = testMaster.mainAccount();
        bob = testMaster.newEmptyAccount();

        tokenDetails = testMaster.environment().erc20Token;
        aliceErc20 = new zksync.Contract(tokenDetails.l2Address, zksync.utils.IERC20, alice);
    });

    test('Can perform a deposit', async () => {
        const amount = 1;
        const gasPrice = scaledGasPrice(alice);

        const initialTokenBalance = await alice.getBalanceL1(tokenDetails.l1Address);
        await alice.deposit({
            token: tokenDetails.l1Address,
            amount,
            approveERC20: true,
            approveOverrides: {
                gasPrice
            },
            overrides: {
                gasPrice
            }
        });

        const finalTokenBalance = await alice.getBalanceL1(tokenDetails.l1Address);
        expect(finalTokenBalance.sub(initialTokenBalance)).toEqual(amount);
    });

    afterAll(async () => {
        await testMaster.deinitialize();
    });
});
