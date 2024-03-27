const zksync = require('zksync-ethers');
const ethers = require('ethers');
const { token, l2BaseTokenAddress, privateKye, l1Provider, l2Provider, alice, amount } = require('./constants');

async function deposit() {
    const gasPrice = 100;
    console.log('Using address: ', alice.address);
    console.log('Initial balances before deposit');

    const initialEthBalance = await alice.getBalanceL1();
    console.log('L1 Ethereum', ethers.utils.formatEther(initialEthBalance));

    const initialL1Balance = await alice.getBalanceL1(token);
    console.log('L1 Base Token', ethers.utils.formatEther(initialL1Balance));

    const initialL2Balance = await alice.getBalance();
    console.log('L2 Base Token', ethers.utils.formatEther(initialL2Balance));

    console.log("Starting deposit of amount: ", ethers.utils.formatUnits(amount, 18));

    const depositTx = await alice.deposit({
        token: token,
        amount: amount,
        approveERC20: true,
        approveBaseERC20: true
    });
    const depositHash = depositTx.hash;
    await depositTx.wait();

    const receipt = await alice._providerL1().getTransactionReceipt(depositHash);
    const fee = receipt.effectiveGasPrice.mul(receipt.gasUsed);

    console.log('Deposit sucessful with tx hash: ', depositHash);

    console.log('\n\nFinal balances');

    const finalEthBalance = await alice.getBalanceL1();
    console.log('L1 Ethereum', ethers.utils.formatEther(finalEthBalance));

    const finalL1Balance = await alice.getBalanceL1(token);
    console.log('L1 Base Token', ethers.utils.formatEther(finalL1Balance));

    const finalL2Balance = await alice.getBalance();
    console.log('L2 Base Token', ethers.utils.formatEther(finalL2Balance));
    return depositHash;
}
(async () => {
    await deposit();
})();
