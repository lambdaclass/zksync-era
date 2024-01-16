/**
 * This suite contains tests checking deposits.
 * Should have 2 main tests:
 1. One that does a regular valid deposit and checks that:
    - The native balance on the L2 increase by the amount deposited.
    - The ERC20 balance on the L1 decreased by that same amount plus a bit more (accounting for the operator fee).
    - The eth balance on the L1 decreased, but only to cover the deposit transaction fee on the L1.
 2. One that ensures that no one can deposit more money than they have.
 */

import { TestMaster } from '../src/index';
import { Token } from '../src/types';
import { shouldChangeTokenBalances, shouldOnlyTakeFee } from '../src/modifiers/balance-checker';

import * as zksync from 'zksync-web3';
import { BigNumber, utils as etherUtils } from 'ethers';
import * as ethers from 'ethers';
import { scaledGasPrice, waitUntilBlockFinalized } from '../src/helpers';
import { L2_ETH_PER_ACCOUNT } from '../src/context-owner';

describe('Deposit', () => {
    let testMaster: TestMaster;
    let alice: zksync.Wallet;
    let bob: zksync.Wallet;
    let tokenDetails: Token;
    let aliceErc20: zksync.Contract;

    beforeAll(async () => {
        testMaster = TestMaster.getInstance(__filename); // Configures env vars for the test.
        alice = testMaster.mainAccount(); // funded amount.
        bob = testMaster.newEmptyAccount(); // empty account.

        tokenDetails = testMaster.environment().erc20Token; // Contains the native token details.
        aliceErc20 = new zksync.Contract(tokenDetails.l2Address, zksync.utils.IERC20, alice); //
    });

    test('Token properties are correct', async () => {
        // expect(aliceErc20.name()).resolves.toBe(tokenDetails.name);
        // expect(aliceErc20.decimals()).resolves.toBe(tokenDetails.decimals);
        // expect(aliceErc20.symbol()).resolves.toBe(tokenDetails.symbol);
        // expect(aliceErc20.balanceOf(alice.address)).resolves.bnToBeGt(0, 'Alice should have non-zero balance');
    });

    test('Can perform a deposit', async () => {
        const amount = 1;
        const gasPrice = scaledGasPrice(alice);

        const initialEthBalanceL1 = await alice.getBalanceL1(tokenDetails.l1Address);
        console.log('alice inicital balance', initialEthBalanceL1.toString());

        console.log('alice L2 address', alice.address);
        const deposit = await alice.deposit({
            token: tokenDetails.l1Address,
            amount,
            approveERC20: true,
            approveOverrides: {
                gasPrice
            },
            overrides: {
                gasPrice
            }
        }, tokenDetails.l1Address);
        console.log('deposit', deposit);
        const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

        await sleep(3000);
        const finalEthBalanceL1 = await alice.getBalanceL1(tokenDetails.l1Address);
        console.log('alice final balance', finalEthBalanceL1.toString());
        
        // const finalTokenBalanceL1 = await aliceErc20.getBalance(alice.address);
        // const finalNativeTokenBalanceL2 = await alice.getBalance(tokenDetails.l2Address);
        
        expect(finalEthBalanceL1).bnToBeEq(initialEthBalanceL1.sub(amount));
        // expect(finalTokenBalanceL1).bnToBeEq(initialTokenBalanceL1.sub(amount));
        // expect(finalNativeTokenBalanceL2).bnToBeEq(initialNativeTokenBalanceL2.add(amount));
    });

    afterAll(async () => {
        await testMaster.deinitialize();
    });
});
