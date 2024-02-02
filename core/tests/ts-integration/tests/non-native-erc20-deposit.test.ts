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
        const amount = BigNumber.from(555);
        const gasPrice = scaledGasPrice(alice);

        // L1 Deposit

        const initialTokenBalance = await alice.getBalanceL1(tokenDetails.l1Address);
        const deposit = await alice.deposit({
            l2GasLimit: 500_000,
            token: tokenDetails.l1Address,
            amount,
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

        const finalTokenBalance = await alice.getBalanceL1(tokenDetails.l1Address);
        console.log('initialTokenBalance', initialTokenBalance.toString());
        console.log('finalTokenBalance', finalTokenBalance.toString());

        // L2 Deposit Finalize
        const l2ERC20Bridge = (await alice.getL2BridgeContracts()).erc20;
        const finalizeDeposit = await l2ERC20Bridge['finalizeDeposit(address,address,address,uint256,bytes)'](
            alice.address,
            alice.address,
            tokenDetails.l1Address,
            amount,
            '0x'
        );
        console.log('wait finalize', await finalizeDeposit.wait());

        // L2 balance with address trough l2 bridge address
        const l2TokenAddressWithL2Bridge = await l2ERC20Bridge.l2TokenAddress(tokenDetails.l1Address, {});
        const l2BalanceThroughL2Bridge = await alice.getBalance(l2TokenAddressWithL2Bridge);
        console.log('[ADDRESS] l2TokenAddressWithL2Bridge: ', l2TokenAddressWithL2Bridge);
        console.log('[BALANCE] l2BalanceThroughL2Bridge with l2 bridge address', l2BalanceThroughL2Bridge.toString());

        // L2 balance with address trough l1 address
        const l2TokenAddress = await (
            await alice.getL1BridgeContracts()
        ).erc20['l2TokenAddress(address)'](tokenDetails.l1Address);
        const l2BalanceThroughL1Bridge = await alice.getBalance(l2TokenAddress);
        console.log('[ADDRESS] l2BalanceThroughL1Bridge with l1 address', l2BalanceThroughL1Bridge.toString());
        console.log('[BALANCE] l2BalanceThroughL1Bridge with l1 address', l2BalanceThroughL1Bridge.toString());

        // L2 balance with l1 address
        const l2Balance = await alice.getBalance(tokenDetails.l1Address);
        console.log('[ADDRESS] l2Balance with l1 address', tokenDetails.l1Address.toString());
        console.log('[BALANCE] l2Balance with l1 address', l2Balance.toString());

        // L2 balance with l2 address
        const l2Balance2 = await alice.getBalance(tokenDetails.l2Address);
        console.log('[ADDRESS] l2Balance with l2 address', tokenDetails.l2Address.toString());
        console.log('[BALANCE] l2Balance with l2 address', l2Balance2.toString());

        // L2 balance with l2 address
        const aliceL2Balance = await alice.getBalance(aliceErc20.address);
        console.log('[ADDRESS] alice l2Balance with l2 address', aliceErc20.address.toString());
        console.log('[BALANCE] alice l2Balance with l2 address', aliceL2Balance.toString());

        expect(initialTokenBalance.sub(finalTokenBalance)).toEqual(BigNumber.from(amount));
    });

    afterAll(async () => {
        await testMaster.deinitialize();
    });
});
