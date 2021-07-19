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

//! Precompile to call pallet-crowdloan-rewards runtime methods via the EVM

#![cfg_attr(not(feature = "std"), no_std)]

use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use frame_support::{
	dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo},
	traits::{Currency, Get},
};
use pallet_evm::{AddressMapping, GasWeightMapping, Precompile};
use sp_core::{H160, U256};
use sp_std::{
	convert::{TryFrom, TryInto},
	fmt::Debug,
	marker::PhantomData,
	vec::Vec,
};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type BalanceOf<Runtime> =
	<<Runtime as pallet_crowdloan_rewards::Config>::RewardCurrency as Currency<
		<Runtime as frame_system::Config>::AccountId,
	>>::Balance;

/// A precompile to wrap the functionality from pallet_crowdloan_rewards.
///
/// EXAMPLE USECASE:
/// A simple example usecase is a contract that allows donors to donate, and stakes all the funds
/// toward one fixed address chosen by the deployer.
/// Such a contract could be deployed by a collator candidate, and the deploy address distributed to
/// supporters who want to donate toward a perpetual nomination fund.
pub struct CrowdloanRewardsWrapper<Runtime>(PhantomData<Runtime>);

impl<Runtime> Precompile for CrowdloanRewardsWrapper<Runtime>
where
	Runtime: pallet_crowdloan_rewards::Config + pallet_evm::Config,
	Runtime::AccountId: From<H160>,
	BalanceOf<Runtime>: TryFrom<U256> + Debug,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
	Runtime::Call: From<pallet_crowdloan_rewards::Call<Runtime>>,
{
	fn execute(
		input: &[u8], //Reminder this is big-endian
		target_gas: Option<u64>,
		context: &Context,
	) -> Result<PrecompileOutput, ExitError> {
		log::trace!(target: "crowdloan-rewards-precompile", "In crowdloan rewards wrapper");

		// Basic sanity checking for length
		// https://solidity-by-example.org/primitives/

		const SELECTOR_SIZE_BYTES: usize = 4;

		if input.len() < 4 {
			return Err(ExitError::Other("input length less than 4 bytes".into()));
		}

		log::trace!(target: "crowdloan-rewards-precompile", "Made it past preliminary length check");
		log::trace!(target: "crowdloan-rewards-precompile", "context.caller is {:?}", context.caller);

		// Parse the function selector
		// These are the four-byte function selectors calculated from the CrowdloanInterface.sol
		// according to the solidity specification
		// https://docs.soliditylang.org/en/v0.8.0/abi-spec.html#function-selector
		let inner_call = match input[0..SELECTOR_SIZE_BYTES] {
			// Check for accessor methods first. These return results immediatelyç
			[0x53, 0x44, 0x0c, 0x90] => {
				return Self::is_contributor(&input[SELECTOR_SIZE_BYTES..], target_gas);
			}
			[0x76, 0xf7, 0x02, 0x49] => {
				return Self::reward_info(&input[SELECTOR_SIZE_BYTES..], target_gas);
			}
			[0x4e, 0x71, 0xd9, 0x2d] => Self::claim()?,
			_ => {
				log::trace!(
					target: "crowdloan-rewards-precompile",
					"Failed to match function selector in crowdloan rewards precompile"
				);
				return Err(ExitError::Other(
					"No crowdloan rewards wrapper method at given selector".into(),
				));
			}
		};

		let outer_call: Runtime::Call = inner_call.into();
		let info = outer_call.get_dispatch_info();

		// Make sure enough gas
		if let Some(gas_limit) = target_gas {
			let required_gas = Runtime::GasWeightMapping::weight_to_gas(info.weight);
			if required_gas > gas_limit {
				return Err(ExitError::OutOfGas);
			}
		}
		log::trace!(target: "crowdloan-rewards-precompile", "Made it past gas check");

		// Dispatch that call
		let origin = Runtime::AddressMapping::into_account_id(context.caller);

		log::trace!(target: "crowdloan-rewards-precompile", "Gonna call with origin {:?}", origin);

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
					target: "crowdloan-rewards-precompile",
					"Crowdloan rewards call via evm failed {:?}",
					e
				);
				Err(ExitError::Other(
					"Crowdloan rewards call via EVM failed".into(),
				))
			}
		}
	}
}

impl<Runtime> CrowdloanRewardsWrapper<Runtime>
where
	Runtime: pallet_crowdloan_rewards::Config + pallet_evm::Config + frame_system::Config,
	Runtime::AccountId: From<H160>,
	BalanceOf<Runtime>: TryFrom<U256> + TryInto<u128> + Debug,
	Runtime::Call: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
	<Runtime::Call as Dispatchable>::Origin: From<Option<Runtime::AccountId>>,
	Runtime::Call: From<pallet_crowdloan_rewards::Call<Runtime>>,
{
	// The accessors are first. They directly return their result.
	fn is_contributor(
		input: &[u8],
		target_gas: Option<u64>,
	) -> Result<PrecompileOutput, ExitError> {
		// parse the address
		let contributor = H160::from_slice(&input[12..32]);

		log::trace!(
			target: "crowdloan-rewards-precompile",
			"Checking whether {:?} is a contributor",
			contributor
		);

		let gas_consumed = <Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(
			<Runtime as frame_system::Config>::DbWeight::get().read,
		);

		// Make sure enough gas
		if let Some(gas_limit) = target_gas {
			if gas_consumed > gas_limit {
				return Err(ExitError::OutOfGas);
			}
		}

		let account: Runtime::AccountId = contributor.into();
		// fetch data from pallet
		let is_contributor =
			pallet_crowdloan_rewards::Pallet::<Runtime>::accounts_payable(account).is_some();

		log::trace!(target: "crowldoan-rewards-precompile", "Result from pallet is {:?}", is_contributor);

		let gas_consumed = <Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(
			<Runtime as frame_system::Config>::DbWeight::get().read,
		);

		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: gas_consumed,
			output: bool_to_solidity_bytes(is_contributor),
			logs: Default::default(),
		})
	}

	fn reward_info(input: &[u8], target_gas: Option<u64>) -> Result<PrecompileOutput, ExitError> {
		// parse the address
		let contributor = H160::from_slice(&input[12..32]);

		log::trace!(
			target: "crowdloan-rewards-precompile",
			"Checking reward info for {:?}",
			contributor
		);

		let account: Runtime::AccountId = contributor.into();

		let gas_consumed = <Runtime as pallet_evm::Config>::GasWeightMapping::weight_to_gas(
			<Runtime as frame_system::Config>::DbWeight::get().read,
		);

		// Make sure enough gas
		if let Some(gas_limit) = target_gas {
			if gas_consumed > gas_limit {
				return Err(ExitError::OutOfGas);
			}
		}

		// fetch data from pallet
		let reward_info = pallet_crowdloan_rewards::Pallet::<Runtime>::accounts_payable(account);

		let (total, claimed): (U256, U256) = if let Some(reward_info) = reward_info {
			let total_reward: u128 = reward_info.total_reward.try_into().map_err(|_| {
				ExitError::Other("Amount is too large for provided balance type".into())
			})?;
			let claimed_reward: u128 = reward_info.claimed_reward.try_into().map_err(|_| {
				ExitError::Other("Amount is too large for provided balance type".into())
			})?;

			(total_reward.into(), claimed_reward.into())
		} else {
			(0u128.into(), 0u128.into())
		};

		log::trace!(
			target: "crowldoan-rewards-precompile", "Result from pallet is {:?}  {:?}",
			total, claimed
		);

		let mut buffer = [0u8; 64];
		total.to_big_endian(&mut buffer[0..32]);
		claimed.to_big_endian(&mut buffer[32..64]);
		Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			cost: gas_consumed,
			output: buffer.to_vec(),
			logs: Default::default(),
		})
	}

	fn claim() -> Result<pallet_crowdloan_rewards::Call<Runtime>, ExitError> {
		Ok(pallet_crowdloan_rewards::Call::<Runtime>::claim())
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
