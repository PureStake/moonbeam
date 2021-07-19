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

//! Test utilities
use crate as stake;
use crate::{pallet, AwardedPts, Config, InflationInfo, Points, Range};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{GenesisBuild, OnFinalize, OnInitialize},
	weights::Weight,
};
use sp_core::H256;
use sp_io;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill, Percent,
};

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Stake: stake::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const SS58Prefix: u8 = 42;
}
impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type BlockWeights = ();
	type BlockLength = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
}
parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
}
impl pallet_balances::Config for Test {
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 4];
	type MaxLocks = ();
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}
parameter_types! {
	pub const MinBlocksPerRound: u32 = 3;
	pub const DefaultBlocksPerRound: u32 = 5;
	pub const LeaveCandidatesDelay: u32 = 2;
	pub const LeaveNominatorsDelay: u32 = 2;
	pub const RevokeNominationDelay: u32 = 2;
	pub const RewardPaymentDelay: u32 = 2;
	pub const MinSelectedCandidates: u32 = 5;
	pub const MaxNominatorsPerCollator: u32 = 4;
	pub const MaxCollatorsPerNominator: u32 = 4;
	pub const DefaultCollatorCommission: Perbill = Perbill::from_percent(20);
	pub const DefaultParachainBondReservePercent: Percent = Percent::from_percent(30);
	pub const MinCollatorStk: u128 = 10;
	pub const MinNominatorStk: u128 = 5;
	pub const MinNomination: u128 = 3;
}
impl Config for Test {
	type Event = Event;
	type Currency = Balances;
	type MonetaryGovernanceOrigin = frame_system::EnsureRoot<AccountId>;
	type MinBlocksPerRound = MinBlocksPerRound;
	type DefaultBlocksPerRound = DefaultBlocksPerRound;
	type LeaveCandidatesDelay = LeaveCandidatesDelay;
	type LeaveNominatorsDelay = LeaveNominatorsDelay;
	type RevokeNominationDelay = RevokeNominationDelay;
	type RewardPaymentDelay = RewardPaymentDelay;
	type MinSelectedCandidates = MinSelectedCandidates;
	type MaxNominatorsPerCollator = MaxNominatorsPerCollator;
	type MaxCollatorsPerNominator = MaxCollatorsPerNominator;
	type DefaultCollatorCommission = DefaultCollatorCommission;
	type DefaultParachainBondReservePercent = DefaultParachainBondReservePercent;
	type MinCollatorStk = MinCollatorStk;
	type MinCollatorCandidateStk = MinCollatorStk;
	type MinNominatorStk = MinNominatorStk;
	type MinNomination = MinNomination;
	type WeightInfo = ();
}

pub(crate) struct ExtBuilder {
	// endowed accounts with balances
	balances: Vec<(AccountId, Balance)>,
	// [collator, amount]
	collators: Vec<(AccountId, Balance)>,
	// [nominator, collator, nomination_amount]
	nominations: Vec<(AccountId, AccountId, Balance)>,
	// inflation config
	inflation: InflationInfo<Balance>,
}

impl Default for ExtBuilder {
	fn default() -> ExtBuilder {
		ExtBuilder {
			balances: vec![],
			nominations: vec![],
			collators: vec![],
			inflation: InflationInfo {
				expect: Range {
					min: 700,
					ideal: 700,
					max: 700,
				},
				// not used
				annual: Range {
					min: Perbill::from_percent(50),
					ideal: Perbill::from_percent(50),
					max: Perbill::from_percent(50),
				},
				// unrealistically high parameterization, only for testing
				round: Range {
					min: Perbill::from_percent(5),
					ideal: Perbill::from_percent(5),
					max: Perbill::from_percent(5),
				},
			},
		}
	}
}

impl ExtBuilder {
	pub(crate) fn with_balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub(crate) fn with_candidates(mut self, collators: Vec<(AccountId, Balance)>) -> Self {
		self.collators = collators;
		self
	}

	pub(crate) fn with_nominations(
		mut self,
		nominations: Vec<(AccountId, AccountId, Balance)>,
	) -> Self {
		self.nominations = nominations;
		self
	}

	#[allow(dead_code)]
	pub(crate) fn with_inflation(mut self, inflation: InflationInfo<Balance>) -> Self {
		self.inflation = inflation;
		self
	}

	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.expect("Frame system builds valid default genesis config");

		pallet_balances::GenesisConfig::<Test> {
			balances: self.balances,
		}
		.assimilate_storage(&mut t)
		.expect("Pallet balances storage can be assimilated");
		stake::GenesisConfig::<Test> {
			candidates: self.collators,
			nominations: self.nominations,
			inflation_config: self.inflation,
		}
		.assimilate_storage(&mut t)
		.expect("Parachain Staking's storage can be assimilated");

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

pub(crate) fn roll_to(n: u64) {
	while System::block_number() < n {
		Stake::on_finalize(System::block_number());
		Balances::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		Balances::on_initialize(System::block_number());
		Stake::on_initialize(System::block_number());
	}
}

pub(crate) fn last_event() -> Event {
	System::events().pop().expect("Event expected").event
}

pub(crate) fn events() -> Vec<pallet::Event<Test>> {
	System::events()
		.into_iter()
		.map(|r| r.event)
		.filter_map(|e| {
			if let Event::Stake(inner) = e {
				Some(inner)
			} else {
				None
			}
		})
		.collect::<Vec<_>>()
}

// Same storage changes as EventHandler::note_author impl
pub(crate) fn set_author(round: u32, acc: u64, pts: u32) {
	<Points<Test>>::mutate(round, |p| *p += pts);
	<AwardedPts<Test>>::mutate(round, acc, |p| *p += pts);
}

#[test]
fn geneses() {
	ExtBuilder::default()
		.with_balances(vec![
			(1, 1000),
			(2, 300),
			(3, 100),
			(4, 100),
			(5, 100),
			(6, 100),
			(7, 100),
			(8, 9),
			(9, 4),
		])
		.with_candidates(vec![(1, 500), (2, 200)])
		.with_nominations(vec![(3, 1, 100), (4, 1, 100), (5, 2, 100), (6, 2, 100)])
		.build()
		.execute_with(|| {
			assert!(System::events().is_empty());
			// collators
			assert_eq!(Balances::reserved_balance(&1), 500);
			assert_eq!(Balances::free_balance(&1), 500);
			assert!(Stake::is_candidate(&1));
			assert_eq!(Balances::reserved_balance(&2), 200);
			assert_eq!(Balances::free_balance(&2), 100);
			assert!(Stake::is_candidate(&2));
			// nominators
			for x in 3..7 {
				assert!(Stake::is_nominator(&x));
				assert_eq!(Balances::free_balance(&x), 0);
				assert_eq!(Balances::reserved_balance(&x), 100);
			}
			// uninvolved
			for x in 7..10 {
				assert!(!Stake::is_nominator(&x));
			}
			assert_eq!(Balances::free_balance(&7), 100);
			assert_eq!(Balances::reserved_balance(&7), 0);
			assert_eq!(Balances::free_balance(&8), 9);
			assert_eq!(Balances::reserved_balance(&8), 0);
			assert_eq!(Balances::free_balance(&9), 4);
			assert_eq!(Balances::reserved_balance(&9), 0);
		});
	ExtBuilder::default()
		.with_balances(vec![
			(1, 100),
			(2, 100),
			(3, 100),
			(4, 100),
			(5, 100),
			(6, 100),
			(7, 100),
			(8, 100),
			(9, 100),
			(10, 100),
		])
		.with_candidates(vec![(1, 20), (2, 20), (3, 20), (4, 20), (5, 10)])
		.with_nominations(vec![
			(6, 1, 10),
			(7, 1, 10),
			(8, 2, 10),
			(9, 2, 10),
			(10, 1, 10),
		])
		.build()
		.execute_with(|| {
			assert!(System::events().is_empty());
			// collators
			for x in 1..5 {
				assert!(Stake::is_candidate(&x));
				assert_eq!(Balances::free_balance(&x), 80);
				assert_eq!(Balances::reserved_balance(&x), 20);
			}
			assert!(Stake::is_candidate(&5));
			assert_eq!(Balances::free_balance(&5), 90);
			assert_eq!(Balances::reserved_balance(&5), 10);
			// nominators
			for x in 6..11 {
				assert!(Stake::is_nominator(&x));
				assert_eq!(Balances::free_balance(&x), 90);
				assert_eq!(Balances::reserved_balance(&x), 10);
			}
		});
}
