const zksync = require('zksync-ethers');
const ethers = require('ethers');
const token = '0x8E9C82509488eD471A83824d20Dd474b8F534a0b';
const l2BaseTokenAddress = '0x000000000000000000000000000000000000800a';
const privateKey = '0x7726827caac94a7f9e1b160f7ea819f172f7b6f9d2a97f992c38edeab82d4110';
const l1Provider = new ethers.providers.JsonRpcProvider('http://127.0.0.1:8545');
const l2Provider = new zksync.Provider('http://127.0.0.1:3050');
const alice = new zksync.Wallet(privateKey, l2Provider, l1Provider);
let toTransfer = 1000000000000000000n;
toTransfer *= 10n;
const amount = ethers.BigNumber.from(toTransfer);
module.exports = Object.freeze({
    token,
    l2BaseTokenAddress,
    privateKey,
    l1Provider,
    l2Provider,
    alice,
    amount
});
