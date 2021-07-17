// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.8.0;

/// The interface through which solidity contracts will interact with pallet-democracy
///
/// This interface does not exhaustively wrap pallet democracy, rather it wraps the most
/// important parts and the parts that are expected to be most useful to evm contracts.
/// More exhaustive wrapping can be added later if it is desireable and the pallet interface
/// is deemed sufficiently stable.
interface Democracy {
    // First some simple accessors

    /// Get the total number of public proposals past and present
    function public_prop_count() external view returns (uint256);

    // Now the dispatchables

    /// Make a new proposal with the given hash locking the given value
    function propose (bytes32 proposal_hash, uint256 value) external;

    /// Signals agreement with a particular proposal
    function second(uint256 proposal, uint256 seconds_upper_bound) external;

    ///
    function vote

    /// function remove_vote

    ///
    function delegate

    ///
    function un_delegate

    ///
    function unlock
}

// These are the selectors generated by remix following this advice
// https://ethereum.stackexchange.com/a/73405/9963
// Eventually we will probably want a better way of generating these and copying them to Rust
// {
// 	"7824e7d1": "propose(bytes32,uint256)",
// 	"56fdf547": "public_prop_count()",
// 	"c7a76601": "second(uint256,uint256)"
// }