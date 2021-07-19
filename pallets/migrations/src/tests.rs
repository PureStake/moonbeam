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

//! Unit testing
use crate::mock::{
	events, ExtBuilder, Migrations, System, MockMigrationManager,
};
use crate::Event;
use std::sync::{Arc, Mutex};
use frame_support::{
	traits::{OnRuntimeUpgrade, OnInitialize},
	weights::{constants::RocksDbWeight, Weight},
};
use sp_runtime::Perbill;

#[test]
fn genesis_builder_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert!(System::events().is_empty());
	})
}

#[test]
fn mock_migrations_static_hack_works() {

	let name_fn_called = Arc::new(Mutex::new(false));
	let step_fn_called = Arc::new(Mutex::new(false));
	let ecb_fn_called = Arc::new(Mutex::new(false));

	crate::mock::execute_with_mock_migrations(&mut |mgr: &mut MockMigrationManager| {
		let name_fn_called = Arc::clone(&name_fn_called);
		let step_fn_called = Arc::clone(&step_fn_called);

		mgr.register_callback(
			move || {
				*name_fn_called.lock().unwrap() = true;
				"hello, world"
			},
			move |_, _| -> (Perbill, Weight) {
				*step_fn_called.lock().unwrap() = true;
				(Perbill::one(), 0u64.into())
			}
		);
	},
	&mut || {
		ExtBuilder::default().build().execute_with(|| {
			crate::mock::roll_until_upgraded(true);
		});
		*ecb_fn_called.lock().unwrap() = true;
	});

	assert_eq!(*name_fn_called.lock().unwrap(), true, "mock migration should call friendly_name()");
	assert_eq!(*step_fn_called.lock().unwrap(), true, "mock migration should call step()");
	assert_eq!(*ecb_fn_called.lock().unwrap(), true, "mock migration should call ECB callback");
}

#[test]
fn on_runtime_upgrade_returns() {
	ExtBuilder::default().build().execute_with(|| {
		Migrations::on_runtime_upgrade();
	})
}

#[test]
fn on_runtime_upgrade_emits_events() {
	ExtBuilder::default().build().execute_with(|| {
		Migrations::on_runtime_upgrade();

		let expected = vec![
			Event::RuntimeUpgradeStarted(),
			Event::RuntimeUpgradeStepped(0u64.into()),
			Event::RuntimeUpgradeCompleted(),
		];
		assert_eq!(events(), expected);
	});
}

#[test]
fn step_called_until_done() {

	let num_step_calls = Arc::new(Mutex::new(0usize));

	crate::mock::execute_with_mock_migrations(&mut |mgr: &mut MockMigrationManager| {
		let num_step_calls = Arc::clone(&num_step_calls);

		mgr.register_callback(
			move || {
				"migration1"
			},
			move |_, _| -> (Perbill, Weight) {
				let mut num_step_calls = num_step_calls.lock().unwrap();
				*num_step_calls += 1;
				if *num_step_calls == 10 {
					(Perbill::one(), 0u64.into())
				} else {
					(Perbill::zero(), 0u64.into())
				}
			}
		);
	},
	&mut || {
		ExtBuilder::default().build().execute_with(|| {
			crate::mock::roll_until_upgraded(true);
		});
	});

	assert_eq!(*num_step_calls.lock().unwrap(), 10, "migration step should be called until done");
}

#[test]
fn migration_progress_should_emit_events() {

	let num_steps = Arc::new(Mutex::new(0usize));

	crate::mock::execute_with_mock_migrations(&mut |mgr: &mut MockMigrationManager| {
		let num_steps = Arc::clone(&num_steps);

		mgr.register_callback(
			move || {
				"migration1"
			},
			move |_, _| -> (Perbill, Weight) {
				let mut num_steps = num_steps.lock().unwrap();

				let result: (Perbill, Weight) = match *num_steps {
					0 => (Perbill::from_percent(50), 50),
					1 => (Perbill::from_percent(60), 51),
					2 => (Perbill::from_percent(70), 52),
					3 => (Perbill::from_percent(80), 53),
					4 => (Perbill::from_percent(100), 1),
					_ => { unreachable!(); }
				};

				*num_steps += 1;
				result
			}
		);
	},
	&mut || {
		ExtBuilder::default().build().execute_with(|| {
			crate::mock::roll_until_upgraded(true);

			let expected = vec![
				Event::RuntimeUpgradeStarted(),
				Event::MigrationStarted("migration1".into()),
				Event::MigrationStepped("migration1".into(), Perbill::from_percent(50), 50),
				Event::RuntimeUpgradeStepped(50),
				Event::MigrationStepped("migration1".into(), Perbill::from_percent(60), 51),
				Event::RuntimeUpgradeStepped(51),
				Event::MigrationStepped("migration1".into(), Perbill::from_percent(70), 52),
				Event::RuntimeUpgradeStepped(52),
				Event::MigrationStepped("migration1".into(), Perbill::from_percent(80), 53),
				Event::RuntimeUpgradeStepped(53),
				Event::MigrationStepped("migration1".into(), Perbill::from_percent(100), 1),
				Event::MigrationCompleted("migration1".into()),
				Event::RuntimeUpgradeStepped(1),
				Event::RuntimeUpgradeCompleted(),
			];
			assert_eq!(events(), expected);
		});
	});

	assert_eq!(*num_steps.lock().unwrap(), 5, "migration step should be called until done");

}

#[test]
fn migration_should_only_be_invoked_once() {

	let num_name_fn_calls = Arc::new(Mutex::new(0usize));
	let num_step_fn_calls = Arc::new(Mutex::new(0usize));

	crate::mock::execute_with_mock_migrations(&mut |mgr: &mut MockMigrationManager| {
		let num_name_fn_calls = Arc::clone(&num_name_fn_calls);
		let num_step_fn_calls = Arc::clone(&num_step_fn_calls);

		mgr.register_callback(
			move || {
				let mut num_name_fn_calls = num_name_fn_calls.lock().unwrap();
				*num_name_fn_calls += 1;
				"migration1"
			},
			move |_, _| -> (Perbill, Weight) {
				let mut num_step_fn_calls = num_step_fn_calls.lock().unwrap();
				*num_step_fn_calls += 1;
				(Perbill::one(), 1) // immediately done
			}
		);
	},
	&mut || {
		ExtBuilder::default().build().execute_with(|| {
			// roll forward until upgraded, should happen before block even increments
			crate::mock::roll_until_upgraded(true);

			assert_eq!(System::block_number(), 1);
			assert_eq!(*num_name_fn_calls.lock().unwrap(), 1, "migration name needed once");
			assert_eq!(*num_step_fn_calls.lock().unwrap(), 1, "migration step needed once");
			let mut expected = vec![
				Event::RuntimeUpgradeStarted(),
				Event::MigrationStarted("migration1".into()),
				Event::MigrationStepped("migration1".into(), Perbill::one(), 1),
				Event::MigrationCompleted("migration1".into()),
				Event::RuntimeUpgradeStepped(1),
				Event::RuntimeUpgradeCompleted(),
			];
			assert_eq!(events(), expected);
			
			// attempt to roll forward again, block should still not increment, and migration
			// name fn should be called but pallet_migrations should immediately recognize that
			// no work needs to be done (and not call step)
			crate::mock::roll_until_upgraded(true);

			assert_eq!(System::block_number(), 1);
			assert_eq!(*num_name_fn_calls.lock().unwrap(), 2, "migration name needed twice");
			assert_eq!(*num_step_fn_calls.lock().unwrap(), 1, "migration step not needed again");
			expected.append(&mut vec![
				Event::RuntimeUpgradeStarted(),
				Event::RuntimeUpgradeStepped(0),
				Event::RuntimeUpgradeCompleted(),
			]);
			assert_eq!(events(), expected);

			// roll forward a few blocks
			crate::mock::roll_to(3, false);
			assert_eq!(*num_name_fn_calls.lock().unwrap(), 2, "migration name not needed again");
			assert_eq!(*num_step_fn_calls.lock().unwrap(), 1, "migration step not needed again");
			// assert that no new events have been emitted
			assert_eq!(events(), expected);
		});
	});
}

#[test]
fn on_initialize_should_charge_at_least_one_db_read() {

	ExtBuilder::default().build().execute_with(|| {
		// first call to on_runtime_upgrade should flag FullyUpgraded as true
		Migrations::on_runtime_upgrade();
		assert_eq!(Migrations::is_fully_upgraded(), true);

		// and subsequent call to on_initialize should do nothing but read this value and return
		let weight = <Migrations as OnInitialize<u64>>::on_initialize(1);
		assert_eq!(weight, RocksDbWeight::get().reads(1));
	})
}

#[test]
fn on_runtime_upgrade_charges_minimum_two_db_writes() {

	ExtBuilder::default().build().execute_with(|| {
		let mut weight = Migrations::on_runtime_upgrade();

		// substrate seems to add a write to this call, so substract one for our logic
		weight -= RocksDbWeight::get().writes(1);

		assert_eq!(weight, RocksDbWeight::get().writes(2));
	})
}
