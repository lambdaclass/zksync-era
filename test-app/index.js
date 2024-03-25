const zksync = require('zksync-ethers');
const ethers = require('ethers');

const token = '0x8E9C82509488eD471A83824d20Dd474b8F534a0b';
const l2BaseTokenAddress = "0x000000000000000000000000000000000000800a";
const privateKey = '0x7726827caac94a7f9e1b160f7ea819f172f7b6f9d2a97f992c38edeab82d4110';
const l1Provider = new ethers.providers.JsonRpcProvider('http://127.0.0.1:8545');
const l2Provider = new zksync.Provider('http://127.0.0.1:3050');
const alice = new zksync.Wallet(privateKey, l2Provider, l1Provider);
const amount = 1;

async function deposit() {

    const gasPrice = 100;

    console.log('Initial balances');

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
        approveBaseERC20: true,
        approveBaseOverrides: {
            gasPrice
        },
        approveOverrides: {
            gasPrice
        },
        overrides: {
            gasPrice
        }
    });
    const depositHash = depositTx.hash;
    await depositTx.wait();

    const receipt = await alice._providerL1().getTransactionReceipt(depositHash);
    const fee = receipt.effectiveGasPrice.mul(receipt.gasUsed);

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
async function withdraw() {
    const withdrawalPromise = alice.withdraw({ token: l2BaseTokenAddress , amount });
    const withdrawalTx = await withdrawalPromise;
    const withdrawalHash = withdrawalTx.hash;
    await withdrawalTx.waitFinalize();
    return withdrawalHash;
}

async function main() {
    const depositHash = await deposit();
    console.log("Deposit sucessful with tx hash: ", depositHash) ;
}

(async () => { await main() })();
