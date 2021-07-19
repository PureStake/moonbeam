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

#![cfg(feature = "runtime-benchmarks")]

//! Benchmarking
use crate::{BalanceOf, Call, Config, Pallet, Range};
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::traits::{Currency, Get, OnFinalize, OnInitialize, ReservableCurrency};
use frame_system::RawOrigin;
use nimbus_primitives::EventHandler;
use sp_runtime::{Perbill, Percent};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

/// Default balance amount is minimum collator stake
fn default_balance<T: Config>() -> BalanceOf<T> {
	<<T as Config>::MinCollatorStk as Get<BalanceOf<T>>>::get()
}

/// Create a funded user.
fn create_funded_user<T: Config>(
	string: &'static str,
	n: u32,
	extra: BalanceOf<T>,
) -> T::AccountId {
	const SEED: u32 = 0;
	let user = account(string, n, SEED);
	let default_balance = default_balance::<T>();
	let total = default_balance + extra;
	T::Currency::make_free_balance_be(&user, total);
	T::Currency::issue(total);
	user
}

/// Create a funded nominator. Base balance is MinCollatorStk == default_balance
/// but the amount for the nomination is MinNominatorStk << MinCollatorStk => the rest + extra
/// is free balance for the returned account.
fn create_funded_nominator<T: Config>(
	string: &'static str,
	n: u32,
	extra: BalanceOf<T>,
	collator: T::AccountId,
	collator_nominator_count: u32,
) -> Result<T::AccountId, &'static str> {
	let user = create_funded_user::<T>(string, n, extra);
	Pallet::<T>::nominate(
		RawOrigin::Signed(user.clone()).into(),
		collator,
		<<T as Config>::MinNominatorStk as Get<BalanceOf<T>>>::get(),
		collator_nominator_count,
		0u32, // first nomination for all calls
	)?;
	Ok(user)
}

/// Create a funded collator. Base amount is MinCollatorStk == default_balance but the
/// last parameter `extra` represents how much additional balance is minted to the collator.
fn create_funded_collator<T: Config>(
	string: &'static str,
	n: u32,
	extra: BalanceOf<T>,
	candidate_count: u32,
) -> Result<T::AccountId, &'static str> {
	let user = create_funded_user::<T>(string, n, extra);
	Pallet::<T>::join_candidates(
		RawOrigin::Signed(user.clone()).into(),
		default_balance::<T>(),
		candidate_count,
	)?;
	Ok(user)
}

const USER_SEED: u32 = 999666;

benchmarks! {
	// MONETARY ORIGIN DISPATCHABLES

	set_staking_expectations {
		let stake_range: Range<BalanceOf<T>> = Range {
			min: 100u32.into(),
			ideal: 200u32.into(),
			max: 300u32.into(),
		};
	}: _(RawOrigin::Root, stake_range)
	verify {
		assert_eq!(Pallet::<T>::inflation_config().expect, stake_range);
	}

	set_inflation {
		let inflation_range: Range<Perbill> = Range {
			min: Perbill::from_perthousand(1),
			ideal: Perbill::from_perthousand(2),
			max: Perbill::from_perthousand(3),
		};

	}: _(RawOrigin::Root, inflation_range)
	verify {
		assert_eq!(Pallet::<T>::inflation_config().annual, inflation_range);
	}

	set_parachain_bond_account {
		let parachain_bond_account: T::AccountId = account("TEST", 0u32, USER_SEED);
	}: _(RawOrigin::Root, parachain_bond_account.clone())
	verify {
		assert_eq!(Pallet::<T>::parachain_bond_info().account, parachain_bond_account);
	}

	set_parachain_bond_reserve_percent {
	}: _(RawOrigin::Root, Percent::from_percent(33))
	verify {
		assert_eq!(Pallet::<T>::parachain_bond_info().percent, Percent::from_percent(33));
	}

	// ROOT DISPATCHABLES

	set_total_selected {}: _(RawOrigin::Root, 100u32)
	verify {
		assert_eq!(Pallet::<T>::total_selected(), 100u32);
	}

	set_collator_commission {}: _(RawOrigin::Root, Perbill::from_percent(33))
	verify {
		assert_eq!(Pallet::<T>::collator_commission(), Perbill::from_percent(33));
	}

	set_blocks_per_round {}: _(RawOrigin::Root, 1200u32)
	verify {
		assert_eq!(Pallet::<T>::round().length, 1200u32);
	}

	// USER DISPATCHABLES

	join_candidates {
		let x in 3..1_000;
		// Worst Case Complexity is insertion into an ordered list so \exists full list before call
		let mut candidate_count = 1u32;
		for i in 2..x {
			let seed = USER_SEED - i;
			let collator = create_funded_collator::<T>(
				"collator",
				seed,
				0u32.into(),
				candidate_count
			)?;
			candidate_count += 1u32;
		}
		let caller: T::AccountId = create_funded_user::<T>("caller", USER_SEED, 0u32.into());
	}: _(RawOrigin::Signed(caller.clone()), default_balance::<T>(), candidate_count)
	verify {
		assert!(Pallet::<T>::is_candidate(&caller));
	}

	// This call schedules the collator's exit and removes them from the candidate pool
	// -> it retains the self-bond and nominator bonds
	leave_candidates {
		let x in 3..1_000;
		// Worst Case Complexity is removal from an ordered list so \exists full list before call
		let mut candidate_count = 1u32;
		for i in 2..x {
			let seed = USER_SEED - i;
			let collator = create_funded_collator::<T>(
				"collator",
				seed,
				0u32.into(),
				candidate_count
			)?;
			candidate_count += 1u32;
		}
		let caller: T::AccountId = create_funded_collator::<T>(
			"caller",
			USER_SEED,
			0u32.into(),
			candidate_count,
		)?;
		candidate_count += 1u32;
	}: _(RawOrigin::Signed(caller.clone()), candidate_count)
	verify {
		assert!(Pallet::<T>::collator_state2(&caller).unwrap().is_leaving());
	}

	go_offline {
		let caller: T::AccountId = create_funded_collator::<T>(
			"collator",
			USER_SEED,
			0u32.into(),
			1u32
		)?;
	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		assert!(!Pallet::<T>::collator_state2(&caller).unwrap().is_active());
	}

	go_online {
		let caller: T::AccountId = create_funded_collator::<T>(
			"collator",
			USER_SEED,
			0u32.into(),
			1u32
		)?;
		Pallet::<T>::go_offline(RawOrigin::Signed(caller.clone()).into())?;
	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		assert!(Pallet::<T>::collator_state2(&caller).unwrap().is_active());
	}

	candidate_bond_more {
		let balance = default_balance::<T>();
		let caller: T::AccountId = create_funded_collator::<T>(
			"collator",
			USER_SEED,
			balance,
			1u32,
		)?;
	}: _(RawOrigin::Signed(caller.clone()), balance)
	verify {
		let expected_bond = balance * 2u32.into();
		assert_eq!(T::Currency::reserved_balance(&caller), expected_bond);
	}

	candidate_bond_less {
		let balance = default_balance::<T>();
		let caller: T::AccountId = create_funded_collator::<T>(
			"collator",
			USER_SEED,
			balance,
			1u32,
		)?;
		Pallet::<T>::candidate_bond_more(RawOrigin::Signed(caller.clone()).into(), balance)?;
	}: _(RawOrigin::Signed(caller.clone()), balance)
	verify {
		assert_eq!(T::Currency::reserved_balance(&caller), balance);
	}

	nominate {
		let max_nominations = <<T as Config>::MaxCollatorsPerNominator as Get<u32>>::get();
		let max_nominators = <<T as Config>::MaxNominatorsPerCollator as Get<u32>>::get();
		let x in 3..<<T as Config>::MaxCollatorsPerNominator as Get<u32>>::get();
		let y in 2..<<T as Config>::MaxNominatorsPerCollator as Get<u32>>::get();
		// Worst Case is full of nominations before calling `nominate`
		let mut collators: Vec<T::AccountId> = Vec::new();
		// Initialize MaxCollatorsPerNominator collator candidates
		for i in 2..x {
			let seed = USER_SEED - i;
			let collator = create_funded_collator::<T>(
				"collator",
				seed,
				0u32.into(),
				collators.len() as u32 + 1u32,
			)?;
			collators.push(collator.clone());
		}
		let bond = <<T as Config>::MinNominatorStk as Get<BalanceOf<T>>>::get();
		let extra = if (bond * (collators.len() as u32 + 1u32).into()) > default_balance::<T>() {
			(bond * (collators.len() as u32 + 1u32).into()) - default_balance::<T>()
		} else {
			0u32.into()
		};
		let caller: T::AccountId = create_funded_user::<T>("caller", USER_SEED, extra.into());
		// Nomination count
		let mut nom_nom_count = 0u32;
		// Nominate MaxCollatorsPerNominators collator candidates
		for col in collators.clone() {
			Pallet::<T>::nominate(
				RawOrigin::Signed(caller.clone()).into(), col, bond, 0u32, nom_nom_count
			)?;
			nom_nom_count += 1u32;
		}
		// Last collator to be nominated
		let collator: T::AccountId = create_funded_collator::<T>(
			"collator",
			USER_SEED,
			0u32.into(),
			collators.len() as u32 + 1u32,
		)?;
		// Worst Case Complexity is insertion into an almost full collator
		let mut col_nom_count = 0u32;
		for i in 1..y {
			let seed = USER_SEED + i;
			let nominator = create_funded_nominator::<T>(
				"nominator",
				seed,
				0u32.into(),
				collator.clone(),
				col_nom_count,
			)?;
			col_nom_count += 1u32;
		}
	}: _(RawOrigin::Signed(caller.clone()), collator, bond, col_nom_count, nom_nom_count)
	verify {
		assert!(Pallet::<T>::is_nominator(&caller));
	}

	leave_nominators {
		let x in 2..<<T as Config>::MaxCollatorsPerNominator as Get<u32>>::get();
		// Worst Case is full of nominations before exit
		let mut collators: Vec<T::AccountId> = Vec::new();
		// Initialize MaxCollatorsPerNominator collator candidates
		for i in 1..x {
			let seed = USER_SEED - i;
			let collator = create_funded_collator::<T>(
				"collator",
				seed,
				0u32.into(),
				collators.len() as u32 + 1u32
			)?;
			collators.push(collator.clone());
		}
		let bond = <<T as Config>::MinNominatorStk as Get<BalanceOf<T>>>::get();
		let need = bond * (collators.len() as u32).into();
		let default_minted = default_balance::<T>();
		let need: BalanceOf<T> = if need > default_minted {
			need - default_minted
		} else {
			0u32.into()
		};
		// Fund the nominator
		let caller: T::AccountId = create_funded_user::<T>("caller", USER_SEED, need);
		let nomination_count = collators.len() as u32;
		// Nomination count
		let mut nom_count = 0u32;
		// Nominate MaxCollatorsPerNominators collator candidates
		for col in collators {
			Pallet::<T>::nominate(
				RawOrigin::Signed(caller.clone()).into(),
				col,
				bond,
				0u32,
				nom_count
			)?;
			nom_count += 1u32;
		}
	}: _(RawOrigin::Signed(caller.clone()), nomination_count)
	verify {
		assert!(Pallet::<T>::nominator_state2(&caller).unwrap().is_leaving());
	}

	revoke_nomination {
		let collator: T::AccountId = create_funded_collator::<T>(
			"collator",
			USER_SEED,
			0u32.into(),
			1u32
		)?;
		let caller: T::AccountId = create_funded_user::<T>("caller", USER_SEED, 0u32.into());
		let bond = <<T as Config>::MinNominatorStk as Get<BalanceOf<T>>>::get();
		Pallet::<T>::nominate(RawOrigin::Signed(
			caller.clone()).into(),
			collator.clone(),
			bond,
			0u32,
			0u32
		)?;
	}: _(RawOrigin::Signed(caller.clone()), collator.clone())
	verify {
		assert_eq!(
			Pallet::<T>::nominator_state2(&caller).unwrap().revocations.0[0],
			collator
		);
	}

	nominator_bond_more {
		let collator: T::AccountId = create_funded_collator::<T>(
			"collator",
			USER_SEED,
			0u32.into(),
			1u32
		)?;
		let caller: T::AccountId = create_funded_user::<T>("caller", USER_SEED, 0u32.into());
		let bond = <<T as Config>::MinNominatorStk as Get<BalanceOf<T>>>::get();
		Pallet::<T>::nominate(
			RawOrigin::Signed(caller.clone()).into(),
			collator.clone(),
			bond,
			0u32,
			0u32
		)?;
	}: _(RawOrigin::Signed(caller.clone()), collator, bond)
	verify {
		let expected_bond = bond * 2u32.into();
		assert_eq!(T::Currency::reserved_balance(&caller), expected_bond);
	}

	nominator_bond_less {
		let collator: T::AccountId = create_funded_collator::<T>(
			"collator",
			USER_SEED,
			0u32.into(),
			1u32
		)?;
		let caller: T::AccountId = create_funded_user::<T>("caller", USER_SEED, 0u32.into());
		let total = default_balance::<T>();
		Pallet::<T>::nominate(RawOrigin::Signed(
			caller.clone()).into(),
			collator.clone(),
			total,
			0u32,
			0u32
		)?;
		let bond_less = <<T as Config>::MinNominatorStk as Get<BalanceOf<T>>>::get();
	}: _(RawOrigin::Signed(caller.clone()), collator, bond_less)
	verify {
		let expected = total - bond_less;
		assert_eq!(T::Currency::reserved_balance(&caller), expected);
	}

	// ON_INITIALIZE

	active_on_initialize {
		// TOTAL SELECTED COLLATORS PER ROUND
		let x in 1..28;
		// NOMINATIONS
		let y in 0..(<<T as Config>::MaxNominatorsPerCollator as Get<u32>>::get() * 28);
		let max_nominators_per_collator =
			<<T as Config>::MaxNominatorsPerCollator as Get<u32>>::get();
		let max_nominations = x * max_nominators_per_collator;
		// y should depend on x but cannot directly, we overwrite y here if necessary to bound it
		let total_nominations: u32 = if max_nominations < y { max_nominations } else { y };
		// INITIALIZE RUNTIME STATE
		let high_inflation: Range<Perbill> = Range {
			min: Perbill::one(),
			ideal: Perbill::one(),
			max: Perbill::one(),
		};
		Pallet::<T>::set_inflation(RawOrigin::Root.into(), high_inflation.clone())?;
		Pallet::<T>::set_total_selected(RawOrigin::Root.into(), 28u32)?;
		// INITIALIZE COLLATOR STATE
		let mut collators: Vec<T::AccountId> = Vec::new();
		let mut collator_count = 1u32;
		for i in 0..x {
			let seed = USER_SEED - i;
			let collator = create_funded_collator::<T>(
				"collator",
				seed,
				default_balance::<T>() * 1_000_000u32.into(),
				collator_count
			)?;
			collators.push(collator);
			collator_count += 1u32;
		}
		// STORE starting balances for all collators
		let collator_starting_balances: Vec<(
			T::AccountId,
			<<T as Config>::Currency as Currency<T::AccountId>>::Balance
		)> = collators.iter().map(|x| (x.clone(), T::Currency::free_balance(&x))).collect();
		// INITIALIZE NOMINATIONS
		let mut col_nom_count: BTreeMap<T::AccountId, u32> = BTreeMap::new();
		collators.iter().for_each(|x| {
			col_nom_count.insert(x.clone(), 0u32);
		});
		let mut nominators: Vec<T::AccountId> = Vec::new();
		let mut remaining_nominations = if total_nominations > max_nominators_per_collator {
			for j in 1..(max_nominators_per_collator + 1) {
				let seed = USER_SEED + j;
				let nominator = create_funded_nominator::<T>(
					"nominator",
					seed,
					default_balance::<T>() * 1_000_000u32.into(),
					collators[0].clone(),
					nominators.len() as u32,
				)?;
				nominators.push(nominator);
			}
			total_nominations - max_nominators_per_collator
		} else {
			for j in 1..(total_nominations + 1) {
				let seed = USER_SEED + j;
				let nominator = create_funded_nominator::<T>(
					"nominator",
					seed,
					default_balance::<T>() * 1_000_000u32.into(),
					collators[0].clone(),
					nominators.len() as u32,
				)?;
				nominators.push(nominator);
			}
			0u32
		};
		col_nom_count.insert(collators[0].clone(), nominators.len() as u32);
		// FILL remaining nominations
		if remaining_nominations > 0 {
			for (col, n_count) in col_nom_count.iter_mut() {
				if n_count < &mut (nominators.len() as u32) {
					// assumes nominators.len() <= MaxNominatorsPerCollator
					let mut open_spots = nominators.len() as u32 - *n_count;
					while open_spots > 0 && remaining_nominations > 0 {
						let caller = nominators[open_spots as usize - 1usize].clone();
						if let Ok(_) = Pallet::<T>::nominate(RawOrigin::Signed(
							caller.clone()).into(),
							col.clone(),
							<<T as Config>::MinNominatorStk as Get<BalanceOf<T>>>::get(),
							*n_count,
							collators.len() as u32, // overestimate
						) {
							*n_count += 1;
							remaining_nominations -= 1;
						}
						open_spots -= 1;
					}
				}
				if remaining_nominations == 0 {
					break;
				}
			}
		}
		// STORE starting balances for all nominators
		let nominator_starting_balances: Vec<(
			T::AccountId,
			<<T as Config>::Currency as Currency<T::AccountId>>::Balance
		)> = nominators.iter().map(|x| (x.clone(), T::Currency::free_balance(&x))).collect();
		// PREPARE RUN_TO_BLOCK LOOP
		let before_running_round_index = Pallet::<T>::round().current;
		let round_length: T::BlockNumber = Pallet::<T>::round().length.into();
		let reward_delay = <<T as Config>::RewardPaymentDelay as Get<u32>>::get() + 2u32;
		let mut now = <frame_system::Pallet<T>>::block_number();
		let mut counter = 0usize;
		let end = Pallet::<T>::round().first + (round_length * reward_delay.into());
		// SET collators as authors for blocks from now - end
		while now < end {
			let author = collators[counter % collators.len()].clone();
			Pallet::<T>::note_author(author);
			<frame_system::Pallet<T>>::on_finalize(<frame_system::Pallet<T>>::block_number());
			<frame_system::Pallet<T>>::set_block_number(
				<frame_system::Pallet<T>>::block_number() + 1u32.into()
			);
			<frame_system::Pallet<T>>::on_initialize(<frame_system::Pallet<T>>::block_number());
			Pallet::<T>::on_initialize(<frame_system::Pallet<T>>::block_number());
			now += 1u32.into();
			counter += 1usize;
		}
		Pallet::<T>::note_author(collators[counter % collators.len()].clone());
		<frame_system::Pallet<T>>::on_finalize(<frame_system::Pallet<T>>::block_number());
		<frame_system::Pallet<T>>::set_block_number(
			<frame_system::Pallet<T>>::block_number() + 1u32.into()
		);
		<frame_system::Pallet<T>>::on_initialize(<frame_system::Pallet<T>>::block_number());
	}: { Pallet::<T>::on_initialize(<frame_system::Pallet<T>>::block_number()); }
	verify {
		// Collators have been paid
		for (col, initial) in collator_starting_balances {
			assert!(T::Currency::free_balance(&col) > initial);
		}
		// Nominators have been paid
		for (col, initial) in nominator_starting_balances {
			assert!(T::Currency::free_balance(&col) > initial);
		}
		// Round transitions
		assert_eq!(Pallet::<T>::round().current, before_running_round_index + reward_delay);
	}

	passive_on_initialize {
		let collator: T::AccountId = create_funded_collator::<T>(
			"collator",
			USER_SEED,
			0u32.into(),
			1u32
		)?;
		let start = <frame_system::Pallet<T>>::block_number();
		Pallet::<T>::note_author(collator.clone());
		<frame_system::Pallet<T>>::on_finalize(start);
		<frame_system::Pallet<T>>::set_block_number(
			start + 1u32.into()
		);
		let end = <frame_system::Pallet<T>>::block_number();
		<frame_system::Pallet<T>>::on_initialize(end);
	}: { Pallet::<T>::on_initialize(end); }
	verify {
		// Round transitions
		assert_eq!(start + 1u32.into(), end);
	}
}

#[cfg(test)]
mod tests {
	use crate::benchmarks::*;
	use crate::mock::Test;
	use frame_support::assert_ok;
	use sp_io::TestExternalities;

	pub fn new_test_ext() -> TestExternalities {
		let t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();
		TestExternalities::new(t)
	}

	#[test]
	fn bench_set_staking_expectations() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_staking_expectations::<Test>());
		});
	}

	#[test]
	fn bench_set_inflation() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_inflation::<Test>());
		});
	}

	#[test]
	fn bench_set_parachain_bond_account() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_parachain_bond_account::<Test>());
		});
	}

	#[test]
	fn bench_set_parachain_bond_reserve_percent() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_parachain_bond_reserve_percent::<Test>());
		});
	}

	#[test]
	fn bench_set_total_selected() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_total_selected::<Test>());
		});
	}

	#[test]
	fn bench_set_collator_commission() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_collator_commission::<Test>());
		});
	}

	#[test]
	fn bench_set_blocks_per_round() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_blocks_per_round::<Test>());
		});
	}

	#[test]
	fn bench_join_candidates() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_join_candidates::<Test>());
		});
	}

	#[test]
	fn bench_leave_candidates() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_leave_candidates::<Test>());
		});
	}

	#[test]
	fn bench_go_offline() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_go_offline::<Test>());
		});
	}

	#[test]
	fn bench_go_online() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_go_online::<Test>());
		});
	}

	#[test]
	fn bench_candidate_bond_more() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_candidate_bond_more::<Test>());
		});
	}

	#[test]
	fn bench_candidate_bond_less() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_candidate_bond_less::<Test>());
		});
	}

	#[test]
	fn bench_nominate() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_nominate::<Test>());
		});
	}

	#[test]
	fn bench_leave_nominators() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_leave_nominators::<Test>());
		});
	}

	#[test]
	fn bench_revoke_nomination() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_revoke_nomination::<Test>());
		});
	}

	#[test]
	fn bench_nominator_bond_more() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_nominator_bond_more::<Test>());
		});
	}

	#[test]
	fn bench_nominator_bond_less() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_nominator_bond_less::<Test>());
		});
	}

	#[test]
	fn bench_active_on_initialize() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_active_on_initialize::<Test>());
		});
	}

	#[test]
	fn bench_passive_on_initialize() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_passive_on_initialize::<Test>());
		});
	}
}

impl_benchmark_test_suite!(
	Pallet,
	crate::benchmarks::tests::new_test_ext(),
	crate::mock::Test
);
