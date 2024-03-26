// SPDX-License-Identifier: MIT

pragma solidity 0.8.24;

import {GettersFacetTest} from "./_Getters_Shared.t.sol";

contract GetTotalBatchesCommittedTest is GettersFacetTest {
    function test() public {
        uint256 expected = 96544567876534567890;
        gettersFacetWrapper.util_setTotalBatchesCommitted(expected);

        uint256 received = gettersFacet.getTotalBatchesCommitted();

        assertEq(expected, received, "Total batches committed is incorrect");
    }
}
