// Copyright 2019-2020 PureStake Inc.
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

//! # Migrations

use frame_support::{pallet_prelude::Get, weights::Weight};
use pallet_migrations::Migration;
use sp_runtime::Perbill;
use sp_std::{
	marker::PhantomData,
	prelude::*,
};
use parachain_staking::migrations::delay_nominator_exits_migration;

/// This module acts as a registry where each migration is defined. Each migration should implement
/// the "Migration" trait declared in the pallet-migrations crate.

#[allow(non_camel_case_types)]
pub struct MM_001_StakingDelayNominatorExitsMigration<Runtime>(PhantomData<Runtime>);
impl<Runtime> Migration for MM_001_StakingDelayNominatorExitsMigration<Runtime>
where
	Runtime: parachain_staking::Config,
{
	fn friendly_name(&self) -> &str {
		"StakingDelayNominatorExitsMigration"
	}
	fn step(&self, _previous_progress: Perbill, _available_weight: Weight) -> (Perbill, Weight) {
		delay_nominator_exits_migration::<Runtime>()
	}
}

pub struct CommonMigrations<Runtime>(PhantomData<Runtime>);
impl<Runtime> Get<Vec<Box<dyn Migration>>> for CommonMigrations<Runtime>
where
	Runtime: parachain_staking::Config,
{
	fn get() -> Vec<Box<dyn Migration>> {
		let mm_001 = MM_001_StakingDelayNominatorExitsMigration::<Runtime>{0: Default::default()};

		// TODO: this is a lot of allocation to do upon every get() call. this *should* be avoided
		// except when pallet_migrations undergoes a runtime upgrade -- but TODO: review
		vec![
			Box::new(mm_001),
		]
	}
}
