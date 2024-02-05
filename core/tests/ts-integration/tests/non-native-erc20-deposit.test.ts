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
        const tokenAddress = '0xF12131d79e026D9882d22e4556706496Bdc287E8';
        const amount = BigNumber.from(555);
        const gasPrice = scaledGasPrice(alice);

        // L1 Deposit
        const initialTokenBalance = await alice.getBalanceL1(tokenAddress);
        const deposit = await alice.deposit({
            l2GasLimit: 500_000,
            token: tokenAddress,
            amount,
            to: alice.address,
            approveERC20: true,
            approveOverrides: {
                gasPrice,
                gasLimit: 5_000_000
            },
            overrides: {
                gasPrice,
                gasLimit: 5_000_000
            }
        });

        await deposit.waitL1Commit();

        const finalTokenBalance = await alice.getBalanceL1(tokenAddress);
        console.log('initialTokenBalance', initialTokenBalance.toString());
        console.log('finalTokenBalance', finalTokenBalance.toString());

        // L2 Deposit Finalize
        const l2ERC20Bridge = (await alice.getL2BridgeContracts()).erc20;
        const finalizeDeposit = await l2ERC20Bridge['finalizeDeposit(address,address,address,uint256,bytes)'](
            alice.address,
            alice.address,
            tokenAddress,
            amount,
            '0x'
        );

        const finalizeDepositWait = await finalizeDeposit.wait();
        console.log('finalizeDepositWait', finalizeDepositWait);

        // Token amount should be deposited to the account in the L2 side.

        /// Try through alice.l2TokenAddress
        const aliceL2TokenAddress = await alice.l2TokenAddress(tokenAddress);
        console.log('[ADDRESS] aliceL2TokenAddress', aliceL2TokenAddress);
        const aliceBalanceThroughAliceL2TokenAddress = await alice.getBalance(aliceL2TokenAddress);
        console.log(
            '[BALANCE] aliceBalanceThroughAliceL2TokenAddress',
            aliceBalanceThroughAliceL2TokenAddress.toString()
        );

        /// Try through l2ERC20Bridge.l2TokenAddress call opt 1
        // const l2TokenAddressL2Bridge = await (
        //     await alice.getL2BridgeContracts()
        // ).erc20['l2TokenAddress(address)'](tokenAddress);
        // const l2BalanceThroughL2Bridge = await alice.getBalance(l2TokenAddressL2Bridge);
        // console.log('[ADDRESS] l2BalanceThroughL2Bridge with l1 address', l2BalanceThroughL2Bridge.toString());
        // console.log('[BALANCE] l2BalanceThroughL2Bridge with l1 address', l2BalanceThroughL2Bridge.toString());

        /// Try through l2ERC20Bridge.l2TokenAddress call opt 2
        // // L2 balance with address trough l2 bridge address
        // const l2TokenAddressWithL2Bridge = await l2ERC20Bridge.l2TokenAddress(tokenAddress, {});
        // const l2BalanceThroughL2Bridge = await alice.getBalance(l2TokenAddressWithL2Bridge);
        // console.log('[ADDRESS] l2TokenAddressWithL2Bridge: ', l2TokenAddressWithL2Bridge);
        // console.log('[BALANCE] l2BalanceThroughL2Bridge with l2 bridge address', l2BalanceThroughL2Bridge.toString());

        /// Try through l1ERC20Bridge.l2TokenAddress call
        const l2TokenAddress = await (
            await alice.getL1BridgeContracts()
        ).erc20['l2TokenAddress(address)'](tokenAddress);
        const l2BalanceThroughL1Bridge = await alice.getBalance(l2TokenAddress);
        console.log('[ADDRESS] l2BalanceThroughL1Bridge with l1 address', l2BalanceThroughL1Bridge.toString());
        console.log('[BALANCE] l2BalanceThroughL1Bridge with l1 address', l2BalanceThroughL1Bridge.toString());

        /// Try through same address than L1 call
        const l2Balance = await alice.getBalance(tokenAddress);
        console.log('[ADDRESS] l2Balance with l1 address', tokenAddress.toString());
        console.log('[BALANCE] l2Balance with l1 address', l2Balance.toString());

        expect(initialTokenBalance.sub(finalTokenBalance)).toEqual(BigNumber.from(amount));
    });

    afterAll(async () => {
        await testMaster.deinitialize();
    });
});
