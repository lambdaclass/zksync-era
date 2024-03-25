import { HardhatRuntimeEnvironment } from 'hardhat/types';
import { Wallet, utils, Provider } from 'zksync-ethers';
import { BigNumber } from 'ethers';
import { ethers } from 'ethers';
const pk = '0x7726827caac94a7f9e1b160f7ea819f172f7b6f9d2a97f992c38edeab82d4110';
const address = '0x36615Cf349d7F6344891B1e7CA7C72883F5dc049';
const l1Provider = new Provider('http://127.0.0.1:8545');
const l2Provider = new Provider('http://127.0.0.1:3050');
const baseTokenAddress = '0x8E9C82509488eD471A83824d20Dd474b8F534a0b';
const l1Erc20ABI = [
    'function balanceOf(address owner) view returns (uint256)',
    'function decimals() view returns (uint8)',
    'function symbol() view returns (string)',
    'function transfer(address to, uint amount) returns (bool)'
];
const BaseTokenL1Contract = new ethers.Contract(baseTokenAddress, l1Erc20ABI, l1Provider);

const createWallet = async (): Promise<Wallet> => {
    let wallet = new Wallet(pk);
    wallet = wallet.connect(l2Provider);
    wallet = wallet.connectToL1(l1Provider);
    return wallet;
};
const getTokenData = async (wallet: Wallet) => {
    const l1Erc20Contract = new ethers.Contract(baseTokenAddress, l1Erc20ABI, wallet);
    const name = await l1Erc20Contract.name();
    return name;
};
const depositPromise = async (wallet: Wallet, amount: BigNumber, nonce: number) => {
    return wallet.deposit({
        token: baseTokenAddress,
        approveBaseERC20: true,
        amount
    });
};
const main = async () => {
    const amount = BigNumber.from(1);
    const wallet = await createWallet();
    console.log('Using wallet with PK: ', pk);
    console.log('Using wallet address for L1 and L2: ', address);
    const symbol = await BaseTokenL1Contract.symbol();
    console.log('Using token', symbol.toString(), 'as base token');
    const l1BalanceBeforeDeposit: BigNumber = await wallet.getBalanceL1(baseTokenAddress);
    const l2BalanceBeforeDeposit: BigNumber = await wallet.getBalance();
    console.log('L1 Balance of token:', l1BalanceBeforeDeposit.toString());
    console.log('L2 Balance of token:', l2BalanceBeforeDeposit.toString());
    const bridgeHub = await wallet.getBridgehubContract();
    const erc20Bridge = await bridgeHub.sharedBridge();
    const nonce = await wallet.getNonce();
    const depositResult = await depositPromise(wallet, amount, nonce);
    console.log('Waiting for deposit to be accepted....');
    await depositResult.waitFinalize();
    console.log('Deposit finished!');
    const l1BalanceAfterDeposit = await wallet.getBalanceL1(baseTokenAddress);
    const l2BalanceAfterDeposit = await wallet.getBalance();
    console.log('L1 Balance after deposit: ', l1BalanceAfterDeposit);
    console.log('L1 Balance delta diff: ', l1BalanceBeforeDeposit.sub(l1BalanceAfterDeposit).toString());
    console.log('L2 Balance after deposit:', l2BalanceAfterDeposit);
    console.log('L2 Balance delta diff: ', l2BalanceAfterDeposit.sub(l2BalanceBeforeDeposit).toString());
};
main().then((x) => x);
