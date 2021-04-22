// SPDX-License-Identifier: GPL-3.0-only

// This file contains the abi for the parachain staking precompiles
// The idea is that we can use this to generate the ABI, and pull the selectors from there.

// @nanocryk I've copied over the doc comments from the pallet. Is that useful to solidity devs?
// Like specifically, will their IDEs show those docs or something? Otherwise, not worth copying.

pragma solidity >=0.8.0;

/// A contract definition that wraps the dispatchables from parachain_staking in
/// Solidity. This will be used to generate the ABI for the precompile wrappers.
/// For now we are not including all the administrative set_* functions
contract ParachainStaking {
    /// Join the set of collator candidates
    function join_candidates(uint256 amount) public {}

    /// Request to leave the set of candidates. If successful, the account is immediately
    /// removed from the candidate pool to prevent selection as a collator, but unbonding is
    /// executed with a delay of `BondDuration` rounds.
    function leave_candidates() public {}

    /// Temporarily leave the set of collator candidates without unbonding
    function go_offline() public {}

    /// Rejoin the set of collator candidates if previously had called `go_offline`
    function go_online() public {}

    /// Bond more for collator candidates
    function candidate_bond_more(uint256 more) public {}

    /// Bond less for collator candidates
    function candidate_bond_less(uint256 less) public {}

    /// If caller is not a nominator, then join the set of nominators
    /// If caller is a nominator, then makes nomination to change their nomination state
    function nominate(address collator, uint256 amount) public {}

    /// Leave the set of nominators and, by implication, revoke all ongoing nominations
    function leave_nominators() public {}

    /// Revoke an existing nomination
    function revoke_nomination(address collator) public {}

    /// Bond more for nominators with respect to a specific collator candidate
    function nominator_bond_more(address candidate, uint256 more) public {}

    /// Bond less for nominators with respect to a specific nominator candidate
    function nominator_bond_less(address candidate, uint256 less) public {}
}

// These are the selectros generated by remix following this advice
// https://ethereum.stackexchange.com/a/73405/9963
// Eventually we will probably want a better way of generating these and copying them to Rust
// {
// 	"8e5080e7": "is_nominator(address)"
//
// 	"ad76ed5a": "join_candidates(uint256)",
// 	"b7694219": "leave_candidates()",
// 	"767e0450": "go_offline()",
// 	"d2f73ceb": "go_online()",
// 	"289b6ba7": "candidate_bond_less(uint256)",
// 	"c57bd3a8": "candidate_bond_more(uint256)",
// 	"82f2c8df": "nominate(address,uint256)",
// 	"e8d68a37": "leave_nominators()",
// 	"4b65c34b": "revoke_nomination(address)"
// 	"f6a52569": "nominator_bond_less(address,uint256)",
// 	"971d44c8": "nominator_bond_more(address,uint256)",
// }
