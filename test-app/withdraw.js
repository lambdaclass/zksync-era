const zksync = require('zksync-ethers');
const ethers = require('ethers');
const { token, l2BaseTokenAddress, privateKye, l1Provider, l2Provider, alice, amount } = require('./constants');

async function withdraw() {
    console.log('Initial balances before withdraw');

    const initialEthBalance = await alice.getBalanceL1();
    console.log('L1 Ethereum', ethers.utils.formatEther(initialEthBalance));

    const initialL1Balance = await alice.getBalanceL1(token);
    console.log('L1 Base Token', ethers.utils.formatUnits(initialL1Balance, 18));

    const initialL2Balance = await alice.getBalance();
    console.log('L2 Base Token', ethers.utils.formatEther(initialL2Balance));

    console.log('Starting withdraw for amount:', ethers.utils.formatUnits(amount, 18));

    const withdrawalPromise = alice.withdraw({ token: l2BaseTokenAddress, amount });
    const withdrawalTx = await withdrawalPromise;
    const withdrawalHash = withdrawalTx.hash;
    await withdrawalTx.waitFinalize();
    const withdrawHash = withdrawalTx.hash;
    console.log('Withdraw sucessful with tx hash', withdrawalHash);
    const finalEthBalance = await alice.getBalanceL1();
    const finalL1Balance = await alice.getBalanceL1(token);
    return withdrawHash;
}
async function finishWithdraw(withdrawalHash) {
    const finalizeWithdrawResult = await alice.finalizeWithdrawal(withdrawalHash);
    await finalizeWithdrawResult.wait();
    const newBalanceL1 = await alice.getBalanceL1(token);
    console.log('Withdraw finalized');
    console.log('New balances:')
    console.log('Final balance on L1:', ethers.utils.formatUnits(newBalanceL1, 18), 'BAT');
    const finalL2Balance = await alice.getBalance();
    console.log('Final balance on L2:', ethers.utils.formatEther(finalL2Balance));
}

(async () => {
    const withdrawHash = await withdraw();
    const finishedWithdraw = await finishWithdraw(withdrawHash);
})();
