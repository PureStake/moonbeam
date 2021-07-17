// Copyright 2019-2021 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

//! Precompile to interact with pallet democracy through an evm precompile.

#![cfg_attr(not(feature = "std"), no_std)]

use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo};
use frame_support::traits::{Currency, Get};
use pallet_democracy::Call as DemocracyCall;
use pallet_evm::AddressMapping;
use pallet_evm::GasWeightMapping;
use pallet_evm::Precompile;
use sp_core::{H160, H256, U256};
use sp_std::convert::{TryFrom, TryInto};
use sp_std::fmt::Debug;
use sp_std::marker::PhantomData;
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type BalanceOf<Runtime> = <<Runtime as pallet_democracy::Config>::Currency as Currency<
	<Runtime as frame_system::Config>::AccountId,
>>::Balance;

/// A precompile to wrap the functionality from pallet democracy.
///
/// Grants evm-based DAOs the right to vote making them first-class citizens.
///
/// EXAMPLE USECASE:
/// A political party that citizens delegate their vote to, and the party votes on their behalf.
pub struct DemocracyWrapper<Runtime>(PhantomData<Runtime>);

impl<Runtime> Precompile for DemocracyWrapper<Runtime>
where
	Runtime: pallet_democracy::Config + pallet_evm::Config,
	// Runtime::AccountId: From<H160>,
	BalanceOf<Runtime>: TryFrom<U256> + Debug,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
	Runtime::Call: From<DemocracyCall<Runtime>>,
	Runtime::Hash: From<H256>,
{
	fn execute(
		input: &[u8], //Reminder this is big-endian
		target_gas: Option<u64>,
		context: &Context,
	) -> Result<PrecompileOutput, ExitError> {
		log::trace!(target: "democracy-precompile", "In democracy wrapper");

		// Basic sanity checking for length
		// https://solidity-by-example.org/primitives/
		const SELECTOR_SIZE_BYTES: usize = 4;

		if input.len() < SELECTOR_SIZE_BYTES {
			return Err(ExitError::Other("input length less than 4 bytes".into()));
		}

		log::trace!(target: "democracy-precompile", "Made it past preliminary length check");

		// Parse the function selector
		// These are the four-byte function selectors calculated from the DemocracyInterface.sol
		// according to the solidity specification
		// https://docs.soliditylang.org/en/v0.8.0/abi-spec.html#function-selector
		let inner_call = match input[0..SELECTOR_SIZE_BYTES] {
			// Check for accessor methods first. These return results immediately
			[0x56, 0xfd, 0xf5, 0x47] => {
				return Self::public_prop_count(&input[SELECTOR_SIZE_BYTES..]);
			}
			// [0x85, 0x45, 0xc8, 0x33] => {
			// 	return Self::is_candidate(&input[SELECTOR_SIZE_BYTES..]);
			// }
			// [0x8f, 0x6d, 0x27, 0xc7] => {
			// 	return Self::is_selected_candidate(&input[SELECTOR_SIZE_BYTES..]);
			// }
			// [0xc9, 0xf5, 0x93, 0xb2] => {
			// 	return Self::min_nomination();
			// }
			// [0x97, 0x99, 0xb4, 0xe7] => {
			// 	return Self::points(&input[SELECTOR_SIZE_BYTES..]);
			// }

			// If not an accessor, check for dispatchables. These calls ready for dispatch below.
			[0x78, 0x24, 0xe7, 0xd1] => Self::propose(&input[SELECTOR_SIZE_BYTES..])?,
			[0xc7, 0xa7, 0x66, 0x01] => Self::second(&input[SELECTOR_SIZE_BYTES..])?,
			_ => {
				log::trace!(
					target: "democracy-precompile",
					"Failed to match function selector in democracy precompile"
				);
				return Err(ExitError::Other(
					"No democracy wrapper method at given selector".into(),
				));
			}
		};

		log::trace!(target: "democracy-precompile", "The inner call is {:?}", inner_call);

		let outer_call: Runtime::Call = inner_call.into();
		let info = outer_call.get_dispatch_info();

		// Make sure enough gas
		if let Some(gas_limit) = target_gas {
			let required_gas = Runtime::GasWeightMapping::weight_to_gas(info.weight);
			if required_gas > gas_limit {
				log::trace!(target: "democracy-precompile",
					"Precompile execution ran out of gas, Needed: {:?}, had: {:?}",
					required_gas, gas_limit
				);
				return Err(ExitError::OutOfGas);
			}
		}
		log::trace!(target: "democracy-precompile", "Made it past gas check");

		// Dispatch that call
		let origin = Runtime::AddressMapping::into_account_id(context.caller);

		log::trace!(target: "democracy-precompile", "Gonna call with origin {:?}", origin);

		match outer_call.dispatch(Some(origin).into()) {
			Ok(post_info) => {
				let gas_used = Runtime::GasWeightMapping::weight_to_gas(
					post_info.actual_weight.unwrap_or(info.weight),
				);
				Ok(PrecompileOutput {
					exit_status: ExitSucceed::Stopped,
					cost: gas_used,
					output: Default::default(),
					logs: Default::default(),
				})
			}
			Err(e) => {
				log::trace!(
					target: "democracy-precompile",
					"Democracy call via evm failed {:?}",
					e
				);
				Err(ExitError::Other("Democracy call via EVM failed".into()))
			}
		}
	}
}

/// Parses an H160 account address from a 256 bit (32 byte) buffer. Only the last 20 bytes are used.
fn parse_account(input: &[u8]) -> Result<H160, ExitError> {
	const PADDING_SIZE_BYTES: usize = 12;
	const ACCOUNT_SIZE_BYTES: usize = 20;
	const TOTAL_SIZE_BYTES: usize = PADDING_SIZE_BYTES + ACCOUNT_SIZE_BYTES;

	if input.len() != TOTAL_SIZE_BYTES {
		log::trace!(target: "democracy-precompile",
			"Unable to parse address. Got {} bytes, expected {}",
			input.len(),
			TOTAL_SIZE_BYTES,
		);
		return Err(ExitError::Other(
			"Incorrect input length for account parsing".into(),
		));
	}

	Ok(H160::from_slice(
		&input[PADDING_SIZE_BYTES..TOTAL_SIZE_BYTES],
	))
}

/// Parses a numeric amount from a 256 bit (32 byte) slice. The numeric result type is generic.
fn parse_amount<NumType: TryFrom<U256>>(input: &[u8]) -> Result<NumType, ExitError> {
	Ok(parse_uint256(input)?
		.try_into()
		.map_err(|_| ExitError::Other("Amount is too large for provided numeric type".into()))?)
}

/// Parses a uint256 value
fn parse_uint256(input: &[u8]) -> Result<U256, ExitError> {
	// In solidity all values are encoded to this width
	const SIZE_BYTES: usize = 32;

	if input.len() != SIZE_BYTES {
		log::trace!(target: "democracy-precompile",
			"Unable to parse uint256. Got {} bytes, expected {}",
			input.len(),
			SIZE_BYTES,
		);
		return Err(ExitError::Other(
			"Incorrect input length for uint256 parsing".into(),
		));
	}

	Ok(U256::from_big_endian(&input[0..SIZE_BYTES]))
}

impl<Runtime> DemocracyWrapper<Runtime>
where
	Runtime: pallet_democracy::Config + pallet_evm::Config + frame_system::Config,
	// Runtime::AccountId: From<H160>,
	BalanceOf<Runtime>: TryFrom<U256> + TryInto<u128> + Debug,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
	Runtime::Call: From<DemocracyCall<Runtime>>,
	Runtime::Hash: From<H256>,
{
	// The accessors are first. They directly return their result.

	fn public_prop_count(input: &[u8]) -> Result<PrecompileOutput, ExitError> {
		// Ensure there is no additional input passed
		if input.len() != 0 {
			return Err(ExitError::Other(
				"Incorrect input length for public_prop_count.".into(),
			));
		}

		// Fetch data from pallet
		let count = pallet_democracy::Pallet::<Runtime>::public_prop_count();
		log::trace!(target: "democracy-precompile", "Result from pallet is {:?}", count);

		let mut output_buffer = [0u8; 32];
		U256::from(count).to_big_endian(&mut output_buffer);

		// Weight of one database read
		let gas_consumed = <Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(
			<Runtime as frame_system::Config>::DbWeight::get().read,
		);

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: gas_consumed,
			output: output_buffer.to_vec(),
			logs: Default::default(),
		})
	}

	// The dispatchable wrappers are next. They return a substrate inner Call ready for dispatch.

	fn propose(input: &[u8]) -> Result<DemocracyCall<Runtime>, ExitError> {
		const HASH_SIZE_BYTES: usize = 32;
		const AMOUNT_SIZE_BYTES: usize = 32;

		if input.len() != HASH_SIZE_BYTES + AMOUNT_SIZE_BYTES {
			return Err(ExitError::Other(
				"Incorrect input length for propose.".into(),
			));
		}

		let proposal_hash = H256::from_slice(&input[0..HASH_SIZE_BYTES]);
		let amount = parse_amount::<BalanceOf<Runtime>>(&input[HASH_SIZE_BYTES..])?;

		log::trace!(target: "democracy-precompile", "Proposing with hash {:?}, and amount {:?}", proposal_hash, amount);

		Ok(DemocracyCall::<Runtime>::propose(
			proposal_hash.into(),
			amount,
		))
	}

	fn second(input: &[u8]) -> Result<DemocracyCall<Runtime>, ExitError> {
		const PROPOSAL_SIZE_BYTES: usize = 32;
		const BOUND_SIZE_BYTES: usize = 32;

		if input.len() != PROPOSAL_SIZE_BYTES + BOUND_SIZE_BYTES {
			return Err(ExitError::Other(
				"Incorrect input length for second.".into(),
			));
		}

		//TODO shouldn't we need type annotations here?
		let proposal_index = parse_amount(&input[0..32])?;
		let seconds_upper_bound = parse_amount(&input[32..])?;

		log::trace!(target: "democracy-precompile", "Seconding proposal {:?}, with bound {:?}", proposal_index, seconds_upper_bound);

		Ok(DemocracyCall::<Runtime>::second(
			proposal_index,
			seconds_upper_bound,
		))
	}
}

// Solidity's bool type is 256 bits as shown by these examples
// https://docs.soliditylang.org/en/v0.8.0/abi-spec.html
// This utility function converts a Rust bool into the corresponding Solidity type
fn bool_to_solidity_bytes(b: bool) -> Vec<u8> {
	let mut result_bytes = [0u8; 32];

	if b {
		result_bytes[31] = 1;
	}

	result_bytes.to_vec()
}

// Solidity;s uint256 (aka uint) type is 32 bytes big endian
fn u256_to_solidity_bytes(n: U256) -> Vec<u8> {
	let mut result_bytes = [0u8; 32];
	n.to_big_endian(&mut result_bytes);
	result_bytes.to_vec()
}
