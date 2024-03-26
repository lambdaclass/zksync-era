const zksync = require('zksync-ethers');
const ethers = require('ethers');
const { token, l2BaseTokenAddress, privateKye, l1Provider, l2Provider, alice, amount } = require('./constants');

async function deposit() {
    const gasPrice = 100;
    console.log('Using address: ', alice.address);
    console.log('Initial balances before deposit');

    const initialEthBalance = await alice.getBalanceL1();
    console.log('L1 Ethereum', initialEthBalance.toString());

    const initialL1Balance = await alice.getBalanceL1(token);
    console.log('L1 Base Token', initialL1Balance.toString());

    const initialL2Balance = await alice.getBalance();
    console.log('L2 Base Token', initialL2Balance.toString());

    const depositTx = await alice.deposit({
        token: token,
        amount: amount,
        approveERC20: true,
        approveBaseERC20: true
    });
    const depositHash = depositTx.hash;
    await depositTx.wait();

    const receipt = await alice._providerL1().getTransactionReceipt(depositHash);
    console.log('The receipt: ', receipt);
    const fee = receipt.effectiveGasPrice.mul(receipt.gasUsed);

    console.log('Deposit sucessful with tx hash: ', depositHash);

    console.log('\n\nFinal balances');

    const finalEthBalance = await alice.getBalanceL1();
    console.log('L1 Ethereum', finalEthBalance.toString());
    console.log('balance diff', initialEthBalance.sub(finalEthBalance).toString());

    const finalL1Balance = await alice.getBalanceL1(token);
    console.log('L1 Base Token', finalL1Balance.toString());
    console.log('balance diff', initialL1Balance.sub(finalL1Balance).toString());

    const finalL2Balance = await alice.getBalance();
    console.log('finalL2Balance', finalL2Balance.toString());
    console.log('L2 Base Token', finalL2Balance.sub(initialL2Balance).toString());
    return depositHash;
}
(async () => {
    await deposit();
})();
