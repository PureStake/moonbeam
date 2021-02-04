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

//! Pallet that sets inflation schedule, used by stake

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_module, decl_storage, ensure};
use frame_system::Config as System;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{DispatchResult, Perbill, RuntimeDebug};

#[derive(Eq, PartialEq, Clone, Encode, Decode, Default, RuntimeDebug)]
pub struct InflationSchedule<T: Ord> {
	pub max: T,
	pub min: T,
	pub ideal: T,
}

impl<T: Ord> InflationSchedule<T> {
	pub fn valid(&self) -> bool {
		self.max > self.ideal && self.ideal > self.min
	}
}

pub trait UpdateInflation {
	fn update_inflation(schedule: InflationSchedule<Perbill>);
}

pub trait Config: System {
	/// For updating dependent storage items
	type Handler: UpdateInflation;
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// Inflation schedule not formed correctly (max <= ideal || min >= ideal)
		InvalidSchedule,
	}
}

decl_storage! {
	trait Store for Module<T: Config> as Inflation {
		/// Annual inflation targets
		pub Schedule get(fn schedule): InflationSchedule<Perbill>;
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {

		#[weight = 0]
		fn set_schedule(origin, schedule: InflationSchedule<Perbill>) -> DispatchResult {
			//ensure_root(origin)?;
			ensure!(schedule.valid(), Error::<T>::InvalidSchedule);
			<Schedule>::put(schedule.clone());
			T::Handler::update_inflation(schedule);
			Ok(())
		}
	}
}
