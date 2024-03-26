const zksync = require('zksync-ethers');
const ethers = require('ethers');
const { token, l2BaseTokenAddress, privateKye, l1Provider, l2Provider, alice, amount } = require('./constants');

async function withdraw() {
    console.log('Initial balances before withdraw');

    const initialEthBalance = await alice.getBalanceL1();
    console.log('L1 Ethereum', initialEthBalance.toString());

    const initialL1Balance = await alice.getBalanceL1(token);
    console.log('L1 Base Token', initialL1Balance.toString());

    const initialL2Balance = await alice.getBalance();
    console.log('L2 Base Token', initialL2Balance.toString());

    console.log('Starting withdraw for amount:', amount);
    const withdrawalPromise = alice.withdraw({ token: l2BaseTokenAddress, amount });
    const withdrawalTx = await withdrawalPromise;
    const withdrawalHash = withdrawalTx.hash;
    await withdrawalTx.waitFinalize();
    const withdrawHash = withdrawalTx.hash;
    console.log('Withdraw sucessful with tx hash', withdrawalHash);

    const finalEthBalance = await alice.getBalanceL1();
    console.log('L1 Ethereum', finalEthBalance.toString());
    console.log('balance diff', initialEthBalance.sub(finalEthBalance).toString());

    const finalL1Balance = await alice.getBalanceL1(token);
    console.log('L1 Base Token', finalL1Balance.toString());
    console.log('balance diff', initialL1Balance.sub(finalL1Balance).toString());

    const finalL2Balance = await alice.getBalance();
    console.log('finalL2Balance', finalL2Balance.toString());
    console.log('L2 Base Token', finalL2Balance.sub(initialL2Balance).toString());
    return withdrawHash;
}
async function finishWithdraw(withdrawalHash) {
    const finalizeWithdrawResult = await alice.finalizeWithdrawal(withdrawalHash);
    await finalizeWithdrawResult.wait();
    const newBalanceL1 = await alice.getBalanceL1(token);
    console.log('Finished withdraw');
    console.log('Final balance on L1: ', newBalanceL1.toString(), 'LBC');
}

(async () => {
    const withdrawHash = await withdraw();
    const finishedWithdraw = await finishWithdraw(withdrawHash);
})();
