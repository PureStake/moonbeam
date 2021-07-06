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

//! # Liquid Staking Module
//!
//! ## Overview
//!
//! Module to provide interaction with Relay Chain Tokens directly
//! This module allows to
//! - Token transfer from parachain to relay chain.
//! - Token transfer from relay to parachain
//! - Exposure to staking functions

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet;

pub use pallet::*;
#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

#[pallet]
pub mod pallet {

	use cumulus_primitives_core::relay_chain;
	use frame_support::dispatch::fmt::Debug;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement::AllowDeath, ReservableCurrency},
		PalletId,
	};
	use frame_system::{ensure_signed, pallet_prelude::*};
	use sp_io::hashing::blake2_256;
	use sp_runtime::traits::AccountIdConversion;
	use sp_runtime::traits::CheckedAdd;
	use sp_runtime::traits::Convert;
	use sp_runtime::AccountId32;
	use sp_runtime::SaturatedConversion;
	use sp_std::prelude::*;

	use substrate_fixed::types::U64F64;
	use xcm::v0::prelude::*;
	use xcm_executor::traits::WeightBounds;

	type BalanceOf<T> =
		<<T as Config>::RelayCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	/// Stores info about how many DOTS someone has staked and the relation with the ratio
	#[derive(Default, Clone, Encode, Decode, RuntimeDebug)]
	pub struct StakeInfo<T: Config> {
		pub staked_without_ratio: BalanceOf<T>,
		pub staked_with_ratio: BalanceOf<T>,
	}

	/// Configuration trait of this pallet. We tightly couple to Parachain Staking in order to
	/// ensure that only staked accounts can create registrations in the first place. This could be
	/// generalized.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency type for Relay balances
		type RelayCurrency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Convert local balance into relay chain balance type
		type ToRelayChainBalance: Convert<BalanceOf<Self>, relay_chain::Balance>;

		/// The Pallets PalletId
		type PalletId: Get<PalletId>;

		type RelayChainAccountId: Parameter
			+ Member
			+ MaybeSerializeDeserialize
			+ Ord
			+ Default
			+ Debug
			+ Into<AccountId32>;

		type SovereignAccount: Get<Self::RelayChainAccountId>;

		/// XCM executor.
		type CallEncoder: EncodeCall<Self>;

		type RelayChainProxyType: Parameter
			+ Member
			+ Ord
			+ PartialOrd
			+ Default
			+ Debug
			+ MaxEncodedLen;

		/// XCM executor.
		type XcmExecutor: ExecuteXcm<Self::Call>;

		/// XCM sender.
		type XcmSender: SendXcm;

		/// Means of measuring the weight consumed by an XCM message locally.
		type Weigher: WeightBounds<Self::Call>;
	}

	/// All possible messages that may be delivered to generic Substrate chain.
	///
	/// Note this enum may be used in the context of both Source (as part of `encode-call`)
	/// and Target chain (as part of `encode-message/send-message`).
	#[derive(Debug, PartialEq, Eq)]
	pub enum AvailableCalls<T: Config> {
		CreateAnonymusProxy(T::RelayChainProxyType, relay_chain::BlockNumber, u16),
		BondThroughAnonymousProxy(T::RelayChainAccountId, relay_chain::Balance),
	}

	pub trait EncodeCall<T: Config> {
		/// Encode call from the relay.
		fn encode_call(call: AvailableCalls<T>) -> Vec<u8>;
	}

	#[pallet::storage]
	#[pallet::getter(fn current_nomination)]
	pub type Nominations<T: Config> = StorageValue<_, Vec<relay_chain::AccountId>, ValueQuery>;

	#[pallet::type_value]
	pub fn RatioDefaultValue<T: Config>() -> U64F64 {
		U64F64::from_num(1)
	}

	#[pallet::storage]
	#[pallet::getter(fn current_ratio)]
	pub type Ratio<T: Config> = StorageValue<_, U64F64, ValueQuery, RatioDefaultValue<T>>;

	#[pallet::storage]
	#[pallet::getter(fn total_staked)]
	pub type TotalStaked<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_staked_multiplier)]
	pub type TotalStakedMultiplier<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn staked_map)]
	pub type StakedMap<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, StakeInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn proxies)]
	pub type Proxies<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::RelayChainAccountId>;

	/// An error that can occur while executing the mapping pallet's logic.
	#[pallet::error]
	pub enum Error<T> {
		MyError,
		WrongConversionU128ToBalance,
		SendFailure,
		Overflow,
		NothingStakedToSetRatio,
		NoRewardsAvailable,
		UnstakingMoreThanStaked,
		ProxyAlreadyCreated,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		Staked(
			<T as frame_system::Config>::AccountId,
			T::RelayChainAccountId,
			BalanceOf<T>,
		),
		Unstaked(<T as frame_system::Config>::AccountId, BalanceOf<T>),
		RatioSet(BalanceOf<T>, BalanceOf<T>),
		NominationsSet(Vec<relay_chain::AccountId>),
		XcmSent(MultiLocation, Xcm<()>),
		ProxyCreated(T::AccountId, T::RelayChainAccountId),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn bond(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
			proxy: T::RelayChainAccountId,
			dest_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Stake bytes
			let amount_as_u128 = amount.saturated_into::<u128>();

			let stake_bytes: Vec<u8> =
				T::CallEncoder::encode_call(AvailableCalls::BondThroughAnonymousProxy(
					proxy.clone(),
					T::ToRelayChainBalance::convert(amount),
				));

			// Construct messages
			let message = Self::transact(
				T::ToRelayChainBalance::convert(amount),
				dest_weight,
				stake_bytes,
			);

			// Send xcm as root
			Self::send_xcm(
				MultiLocation::Null,
				MultiLocation::X1(Parent),
				message.clone(),
			)
			.map_err(|_| Error::<T>::SendFailure)?;

			// Deposit event
			Self::deposit_event(Event::<T>::XcmSent(MultiLocation::Null, message));

			// Deposit event
			Self::deposit_event(Event::<T>::Staked(who.clone(), proxy, amount.clone()));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn create_proxy(
			origin: OriginFor<T>,
			proxy: T::RelayChainProxyType,
			amount: BalanceOf<T>,
			index: u16,
			dest_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let amount_as_u128 = amount.saturated_into::<u128>();

			let stake_bytes: Vec<u8> =
				T::CallEncoder::encode_call(AvailableCalls::CreateAnonymusProxy(proxy, 0, 0));

			// Construct messages
			let message = Self::transact(
				T::ToRelayChainBalance::convert(amount),
				dest_weight,
				stake_bytes,
			);

			// Send xcm as root
			Self::send_xcm(
				MultiLocation::Null,
				MultiLocation::X1(Parent),
				message.clone(),
			)
			.map_err(|_| Error::<T>::SendFailure)?;

			// Deposit event
			Self::deposit_event(Event::<T>::XcmSent(MultiLocation::Null, message));

			// Deposit event
			Self::deposit_event(Event::<T>::ProxyCreated(
				who.clone(),
				T::SovereignAccount::get(),
			));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn unstake_dot(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// We get the current ratio
			let current_ratio = Ratio::<T>::get();
			let total_staked = TotalStaked::<T>::get();

			let total_staked_with_multiplier = TotalStakedMultiplier::<T>::get();

			// We know how much a particular account has on rewards. Its the difference
			// between whatever Let's calculate it.
			let mut staked =
				StakedMap::<T>::get(who.clone()).ok_or(Error::<T>::NoRewardsAvailable)?;
			ensure!(
				amount <= staked.staked_without_ratio,
				Error::<T>::NoRewardsAvailable
			);

			// What portion of the amount are we looking for?
			let amount_to_retrieve =
				(amount * staked.staked_with_ratio) / staked.staked_without_ratio;

			staked.staked_with_ratio -= amount_to_retrieve;
			staked.staked_without_ratio -= amount;

			let v_dot_to_retrieve = (U64F64::from_num(amount_to_retrieve.saturated_into::<u128>())
				* current_ratio)
				.ceil()
				.to_num::<u128>()
				.saturated_into::<BalanceOf<T>>();

			TotalStaked::<T>::put(total_staked - amount);
			TotalStakedMultiplier::<T>::put(total_staked_with_multiplier - amount_to_retrieve);
			StakedMap::<T>::insert(&who, staked);

			if v_dot_to_retrieve > amount {
				T::RelayCurrency::deposit_into_existing(&who, v_dot_to_retrieve - amount)?;
				T::RelayCurrency::transfer(
					&T::PalletId::get().into_account(),
					&who,
					amount,
					AllowDeath,
				)?;
			} else {
				// We need to burn
				let imbalance = T::RelayCurrency::slash(
					&T::PalletId::get().into_account(),
					amount - v_dot_to_retrieve,
				);
				T::RelayCurrency::transfer(
					&T::PalletId::get().into_account(),
					&who,
					v_dot_to_retrieve,
					AllowDeath,
				)?;
			}
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_ratio(origin: OriginFor<T>, dot_in_sovereign: BalanceOf<T>) -> DispatchResult {
			ensure_root(origin)?;
			let total_staked = TotalStaked::<T>::get();
			let total_staked_multiplier = TotalStakedMultiplier::<T>::get();

			// Division by 0
			ensure!(
				total_staked != 0u32.into(),
				Error::<T>::NothingStakedToSetRatio
			);
			let total_issuance: BalanceOf<T> = T::RelayCurrency::total_issuance();
			// The ratio is: the total amount of dots in the sovereign, minus the total issuance of
			// T::RelayCurrency. Those are essentially the dots that were sent to our sovereign but
			// that were not minted in our parachain, i.e., the rewards.
			// The ratio is that difference divided by the total staked

			let difference = dot_in_sovereign - (total_issuance - total_staked);
			// We should be using the total minted (with multiplier)
			let ratio = U64F64::from_num(difference.saturated_into::<u128>())
				/ U64F64::from_num(total_staked_multiplier.saturated_into::<u128>());
			Ratio::<T>::put(ratio);
			Self::deposit_event(Event::<T>::RatioSet(difference, total_staked));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_nominations(
			origin: OriginFor<T>,
			nominations: Vec<relay_chain::AccountId>,
		) -> DispatchResult {
			ensure_root(origin)?;
			<Nominations<T>>::put(nominations.clone());
			Self::deposit_event(Event::<T>::NominationsSet(nominations));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn transact(amount: u128, dest_weight: Weight, call: Vec<u8>) -> Xcm<()> {
			let buy_order = BuyExecution {
				fees: All,
				// Zero weight for additional XCM (since there are none to execute)
				weight: dest_weight,
				debt: dest_weight,
				halt_on_error: false,
				xcm: vec![Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: dest_weight,
					call: call.into(),
				}],
			};

			// We put Null here, as this will be interpreted by the sovereign account
			WithdrawAsset {
				assets: vec![MultiAsset::ConcreteFungible {
					id: MultiLocation::Null,
					amount: amount,
				}],
				effects: vec![buy_order],
			}
		}

		fn send_xcm(
			interior: MultiLocation,
			dest: MultiLocation,
			message: Xcm<()>,
		) -> Result<(), XcmError> {
			let message = match interior {
				MultiLocation::Null => message,
				who => Xcm::<()>::RelayedFrom {
					who,
					message: Box::new(message),
				},
			};
			T::XcmSender::send_xcm(dest, message)
		}
	}
}