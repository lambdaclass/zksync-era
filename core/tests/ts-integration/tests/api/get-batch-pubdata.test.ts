/**
 * This suite contains tests for the getBatchPubdata endpoint.
 */
import { TestMaster } from '../../src';
import * as zksync from 'zksync-web3';
// Regular expression to match variable-length hex number.
const HEX_VALUE_REGEX = /^0x[\da-fA-F]*$/;

describe('getBatchPubdata API tests', () => {
    let testMaster: TestMaster;
    let alice: zksync.Wallet;
    let l2Token: string;

    beforeAll(async () => {
        testMaster = TestMaster.getInstance(__filename);
        alice = testMaster.mainAccount();
        l2Token = testMaster.environment().erc20Token.l2Address;
    });

    test('Should test some zks web3 methods including zks_getBatchPubdata', async () => {
        // zks_getAllAccountBalances
        // NOTE: `getAllBalances` will not work on external node,
        // since TokenListFetcher is not running
        if (!process.env.EN_MAIN_NODE_URL) {
            const balances = await alice.getAllBalances();
            const tokenBalance = await alice.getBalance(l2Token);
            expect(balances[l2Token.toLowerCase()].eq(tokenBalance));
        }
        // zks_getBlockDetails
        const blockDetails = await alice.provider.getBlockDetails(1);
        const block = await alice.provider.getBlock(1);
        expect(blockDetails.rootHash).toEqual(block.hash);
        expect(blockDetails.l1BatchNumber).toEqual(block.l1BatchNumber);
        // zks_getL1BatchDetails
        const batchDetails = await alice.provider.getL1BatchDetails(block.l1BatchNumber);
        expect(batchDetails.number).toEqual(block.l1BatchNumber);
        // zks_getBatchPubdata
        const response = await alice.provider.send('zks_getBatchPubdata', [block.l1BatchNumber]);
        const expectedResponse = {
            gas_limit: expect.stringMatching(HEX_VALUE_REGEX),
            gas_per_pubdata_limit: expect.stringMatching(HEX_VALUE_REGEX),
            max_fee_per_gas: expect.stringMatching(HEX_VALUE_REGEX),
            max_priority_fee_per_gas: expect.stringMatching(HEX_VALUE_REGEX)
        };
        expect(response).toMatchObject(expectedResponse);
    });

    afterAll(async () => {
        await testMaster.deinitialize();
    });
});
