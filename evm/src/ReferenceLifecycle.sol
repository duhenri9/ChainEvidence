// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.28;

/// @notice Test-only reference contract for ChainEvidence V0.3 local EVM evidence.
contract ReferenceLifecycle {
    event LifecycleSet(bytes32 key, bytes32 value);

    function set(bytes32 key, bytes32 value) external {
        emit LifecycleSet(key, value);
    }
}
