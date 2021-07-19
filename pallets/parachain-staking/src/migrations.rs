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

//! # Parachain Staking Migrations
use crate::{
	Config,
	pallet::{
		NominatorState, Nominator2, BalanceOf, NominatorState2, ExitQueue, ExitQueue2, ExitQ,
		OrderedSet,
	},
};
use sp_runtime::Perbill;
use frame_support::weights::Weight;
use sp_std::prelude::*;

/// Storage migration for delaying nomination exits and revocations
pub fn delay_nominator_exits_migration<T: Config>() -> (Perbill, Weight) {

	// note on using pallet_migrations: migrations are not expected to be idempotent but the pallet
	// itself will ensure that they are called only once.

	let weight: Weight = 0_u64.into();

	// migrate from Nominator -> Nominator2
	for (acc, nominator_state) in NominatorState::<T>::drain() {
		let state: Nominator2<T::AccountId, BalanceOf<T>> = nominator_state.into();
		<NominatorState2<T>>::insert(acc, state);

		// TODO: weight += (1 read + 1 write)
	}
	// migrate from ExitQueue -> ExitQueue2
	let just_collators_exit_queue = <ExitQueue<T>>::take();
	let mut candidates: Vec<T::AccountId> = Vec::new();
	for (acc, _) in just_collators_exit_queue.clone().into_iter() {
		candidates.push(acc);
		// TODO: weight += (1 read + 1 write) or so
	}
	<ExitQueue2<T>>::put(ExitQ {
		candidates: candidates.into(),
		nominators_leaving: OrderedSet::new(),
		candidate_schedule: just_collators_exit_queue,
		nominator_schedule: Vec::new(),
	});

	// elaborating to illustrate the purpose of the Perbill
	let progress: Perbill = Perbill::one(); // anything < 1.0 indicates not done

	(progress, weight)
}
