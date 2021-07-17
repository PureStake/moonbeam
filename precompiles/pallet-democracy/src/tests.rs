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

use crate::mock::{
	events, evm_test_context, precompile_address, Call, ExtBuilder, Origin, Precompiles,
	TestAccount::Alice,
};
use crate::{u256_to_solidity_bytes, PrecompileOutput};
use frame_support::{assert_ok, dispatch::Dispatchable};
use pallet_balances::Event as BalancesEvent;
use pallet_democracy::{Call as DemocracyCall, Event as DemocracyEvent};
use pallet_evm::{Call as EvmCall, Event as EvmEvent};
use pallet_evm::{ExitError, ExitSucceed, PrecompileSet};
use sp_core::U256;

#[test]
fn selector_less_than_four_bytes() {
	ExtBuilder::default().build().execute_with(|| {
		// This selector is only three bytes long when four are required.
		let bogus_selector = vec![1u8, 2u8, 3u8];

		// Expected result is an error stating there are too few bytes
		let expected_result = Some(Err(ExitError::Other(
			"input length less than 4 bytes".into(),
		)));

		assert_eq!(
			Precompiles::execute(
				precompile_address(),
				&bogus_selector,
				None,
				&evm_test_context(),
			),
			expected_result
		);
	});
}

#[test]
fn no_selector_exists_but_length_is_right() {
	ExtBuilder::default().build().execute_with(|| {
		// This selector is only three bytes long when four are required.
		let bogus_selector = vec![1u8, 2u8, 3u8, 4u8];

		// Expected result is an error stating there are too few bytes
		let expected_result = Some(Err(ExitError::Other(
			"No democracy wrapper method at given selector".into(),
		)));

		assert_eq!(
			Precompiles::execute(
				precompile_address(),
				&bogus_selector,
				None,
				&evm_test_context(),
			),
			expected_result
		);
	});
}

#[test]
fn prop_count_zero() {
	ExtBuilder::default().build().execute_with(|| {
		let selector = hex_literal::hex!("56fdf547");

		// Construct data to read prop count
		let mut input_data = Vec::<u8>::from([0u8; 4]);
		input_data[0..4].copy_from_slice(&selector);

		// Expected result is zero. because no props are open yet.
		let expected_zero_result = Some(Ok(PrecompileOutput {
			exit_status: ExitSucceed::Returned,
			output: Vec::from([0u8; 32]),
			cost: Default::default(),
			logs: Default::default(),
		}));

		// Assert that no props have been opened.
		assert_eq!(
			Precompiles::execute(precompile_address(), &input_data, None, &evm_test_context()),
			expected_zero_result
		);
	});
}

#[test]
fn prop_count_non_zero() {
	ExtBuilder::default()
		.with_balances(vec![(Alice, 1000)])
		.build()
		.execute_with(|| {
			let selector = hex_literal::hex!("56fdf547");

			// There is no interesting genesis config for pallet democracy so we make the proposal here

			// This line doesn't compile becuase it says `propose` is a private function.
			// Why is this a private function? It is defined as `pub(crate) fn propose`
			// https://github.com/paritytech/substrate/blob/polkadot-v0.9.4/frame/democracy/src/lib.rs#L637
			assert_ok!(
				Call::Democracy(DemocracyCall::propose(Default::default(), 1000u128))
					.dispatch(Origin::signed(Alice))
			);

			// Construct data to read prop count
			let mut input_data = Vec::<u8>::from([0u8; 4]);
			input_data[0..4].copy_from_slice(&selector);

			// Expected result is one
			let expected_one_result = Some(Ok(PrecompileOutput {
				exit_status: ExitSucceed::Returned,
				output: u256_to_solidity_bytes(U256::from(1)),
				cost: Default::default(),
				logs: Default::default(),
			}));

			// Assert that no props have been opened.
			assert_eq!(
				Precompiles::execute(precompile_address(), &input_data, None, &evm_test_context()),
				expected_one_result
			);
		});
}

#[test]
fn prop_count_extra_data() {
	ExtBuilder::default().build().execute_with(|| {
		let selector = hex_literal::hex!("56fdf547");

		// Construct data to read prop count including a bogus extra byte
		let mut input_data = Vec::<u8>::from([0u8; 5]);

		// We still use the correct selector for prop_count
		input_data[0..4].copy_from_slice(&selector);

		// Expected result is an error stating there are too few bytes
		let expected_result = Some(Err(ExitError::Other(
			"Incorrect input length for public_prop_count.".into(),
		)));

		assert_eq!(
			Precompiles::execute(precompile_address(), &input_data, None, &evm_test_context()),
			expected_result
		);
	});
}

#[test]
fn propose_works() {
	ExtBuilder::default()
		.with_balances(vec![(Alice, 1000)])
		.build()
		.execute_with(|| {
			let selector = hex_literal::hex!("7824e7d1");

			// Construct data to propose empty hash with value 100
			let mut input_data = Vec::<u8>::from([0u8; 68]);
			input_data[0..4].copy_from_slice(&selector);
			// Leave the hash (input_data[4..36]) empty
			let amount: U256 = 100.into();
			amount.to_big_endian(&mut input_data[36..68]);

			// Make sure the call goes through successfully
			assert_ok!(Call::Evm(EvmCall::call(
				Alice.to_h160(),
				precompile_address(),
				input_data,
				U256::zero(), // No value sent in EVM
				u64::max_value(),
				0.into(),
				None, // Use the next nonce
			))
			.dispatch(Origin::root()));

			// Assert that the events are as expected
			assert_eq!(
				events(),
				vec![
					BalancesEvent::Reserved(Alice, 100).into(),
					DemocracyEvent::Proposed(0, 100).into(),
					EvmEvent::Executed(precompile_address()).into(),
				]
			);
		})
}

// TODO propose error cases
// propose_bad_length
// proposing fails when you don't have enough funds to cover the deposit

#[test]
fn second_works() {
	ExtBuilder::default()
		.with_balances(vec![(Alice, 1000)])
		.build()
		.execute_with(|| {
			let selector = hex_literal::hex!("c7a76601");

			// Before we can second anything, we have to have a proposal there to second.
			assert_ok!(Call::Democracy(DemocracyCall::propose(
				Default::default(), // Propose the default hash
				100u128,            // bond of 100 tokens
			))
			.dispatch(Origin::signed(Alice)));

			// Construct the call to second via a precompile
			let mut input_data = Vec::<u8>::from([0u8; 68]);
			input_data[0..4].copy_from_slice(&selector);
			let index = U256::zero();
			index.to_big_endian(&mut input_data[4..36]);
			let amount: U256 = 100.into();
			amount.to_big_endian(&mut input_data[36..68]);

			// Make sure the call goes through successfully
			assert_ok!(Call::Evm(EvmCall::call(
				Alice.to_h160(),
				precompile_address(),
				input_data,
				U256::zero(), // No value sent in EVM
				u64::max_value(),
				0.into(),
				None, // Use the next nonce
			))
			.dispatch(Origin::root()));

			// Assert that the events are as expected
			assert_eq!(
				events(),
				vec![
					BalancesEvent::Reserved(Alice, 100).into(),
					DemocracyEvent::Proposed(0, 100).into(),
					// This 100 is reserved for the second.
					// Pallet democracy does not ahve an event for seconding
					BalancesEvent::Reserved(Alice, 100).into(),
					EvmEvent::Executed(precompile_address()).into(),
				]
			);
		})
}

// TODO Second error cases
// proposal doesn't exist
// you can't afford it
