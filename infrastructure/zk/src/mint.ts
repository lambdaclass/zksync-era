import { Command } from 'commander';
import * as utils from './utils';
import { BigNumberish } from 'ethers';

export async function mintRichWallets(wallets: BigNumberish) {
    await utils.spawn(`yarn --cwd contracts/zksync mint-rich-wallets mint --wallets ${wallets}`);
}

export const command = new Command('mint')
    .description('mint rich wallets')
    .option('--wallets <wallets>')
    .action(async (cmd: Command) => {
        mintRichWallets(cmd.wallets);
    });
