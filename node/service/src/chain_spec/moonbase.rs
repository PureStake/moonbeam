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

//! Moonbase Chain Specifications and utilities for building them.
//!
//! Learn more about Substrate chain specifications at
//! https://substrate.dev/docs/en/knowledgebase/integrate/chain-spec

#[cfg(test)]
use crate::chain_spec::{derive_bip44_pairs_from_mnemonic, get_account_id_from_pair};
use crate::chain_spec::{generate_accounts, get_from_seed, Extensions};
use cumulus_primitives_core::ParaId;
use evm::GenesisAccount;
use moonbase_runtime::{
	currency::UNIT, AccountId, AuthorFilterConfig, AuthorMappingConfig, Balance, BalancesConfig,
	CouncilCollectiveConfig, CrowdloanRewardsConfig, DemocracyConfig, EVMConfig,
	EthereumChainIdConfig, EthereumConfig, GenesisConfig, InflationInfo, ParachainInfoConfig,
	ParachainStakingConfig, Precompiles, Range, SchedulerConfig, SudoConfig, SystemConfig,
	TechComitteeCollectiveConfig, WASM_BINARY,
};
use nimbus_primitives::NimbusId;
use sc_service::ChainType;
#[cfg(test)]
use sp_core::ecdsa;
use sp_runtime::Perbill;
use std::str::FromStr;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// Generate a chain spec for use with the development service.
pub fn development_chain_spec(mnemonic: Option<String>, num_accounts: Option<u32>) -> ChainSpec {
	// Default mnemonic if none was provided
	let parent_mnemonic = mnemonic.unwrap_or_else(|| {
		"bottom drive obey lake curtain smoke basket hold race lonely fit walk".to_string()
	});
	// We prefund the standard dev accounts plus Gerald
	let mut accounts = generate_accounts(parent_mnemonic, num_accounts.unwrap_or(10));
	accounts.push(AccountId::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap());

	ChainSpec::from_genesis(
		"Moonbase Development Testnet",
		"moonbase_dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				// Alith is Sudo
				accounts[0],
				// Council members: Baltathar, Charleth and Dorothy
				vec![accounts[1], accounts[2], accounts[3]],
				// Collator Candidate: Alice -> Alith
				vec![(
					accounts[0],
					get_from_seed::<NimbusId>("Alice"),
					1_000 * UNIT,
				)],
				// Nominations
				vec![],
				accounts.clone(),
				3_000_000 * UNIT,
				Default::default(), // para_id
				1281,               //ChainId
			)
		},
		vec![],
		None,
		None,
		Some(
			serde_json::from_str(
				"{\"tokenDecimals\": 18, \"tokenSymbol\": \"UNIT\", \"SS58Prefix\": 1287}",
			)
			.expect("Provided valid json map"),
		),
		Extensions {
			relay_chain: "dev-service".into(),
			para_id: Default::default(),
		},
	)
}

/// Generate a default spec for the parachain service. Use this as a starting point when launching
/// a custom chain.
pub fn get_chain_spec(para_id: ParaId) -> ChainSpec {
	ChainSpec::from_genesis(
		// TODO Apps depends on this string to determine whether the chain is an ethereum compat
		// or not. We should decide the proper strings, and update Apps accordingly.
		// Or maybe Apps can be smart enough to say if the string contains "moonbeam" at all...
		"Moonbase Local Testnet",
		"moonbase_local",
		ChainType::Local,
		move || {
			testnet_genesis(
				// Alith is Sudo
				AccountId::from_str("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac").unwrap(),
				// Council members: Baltathar, Charleth and Dorothy
				vec![
					AccountId::from_str("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0").unwrap(),
					AccountId::from_str("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc").unwrap(),
					AccountId::from_str("773539d4Ac0e786233D90A233654ccEE26a613D9").unwrap(),
				],
				// Collator Candidates
				vec![
					// Alice -> Alith
					(
						AccountId::from_str("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac").unwrap(),
						get_from_seed::<NimbusId>("Alice"),
						1_000 * UNIT,
					),
					// Bob -> Baltithar
					(
						AccountId::from_str("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0").unwrap(),
						get_from_seed::<NimbusId>("Bob"),
						1_000 * UNIT,
					),
				],
				// Nominations
				vec![],
				vec![
					AccountId::from_str("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac").unwrap(),
					AccountId::from_str("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0").unwrap(),
				],
				3_000_000 * UNIT,
				para_id,
				1280, //ChainId
			)
		},
		vec![],
		None,
		None,
		Some(
			serde_json::from_str(
				"{\"tokenDecimals\": 18, \"tokenSymbol\": \"UNIT\", \"SS58Prefix\": 1287}",
			)
			.expect("Provided valid json map"),
		),
		Extensions {
			relay_chain: "westend-local".into(),
			para_id: para_id.into(),
		},
	)
}

pub fn moonbeam_inflation_config() -> InflationInfo<Balance> {
	InflationInfo {
		expect: Range {
			min: 100_000 * UNIT,
			ideal: 200_000 * UNIT,
			max: 500_000 * UNIT,
		},
		annual: Range {
			min: Perbill::from_percent(4),
			ideal: Perbill::from_percent(5),
			max: Perbill::from_percent(5),
		},
		// 8766 rounds (hours) in a year
		round: Range {
			min: Perbill::from_parts(Perbill::from_percent(4).deconstruct() / 8766),
			ideal: Perbill::from_parts(Perbill::from_percent(5).deconstruct() / 8766),
			max: Perbill::from_parts(Perbill::from_percent(5).deconstruct() / 8766),
		},
	}
}

pub fn testnet_genesis(
	root_key: AccountId,
	council_members: Vec<AccountId>,
	candidates: Vec<(AccountId, NimbusId, Balance)>,
	nominations: Vec<(AccountId, AccountId, Balance)>,
	endowed_accounts: Vec<AccountId>,
	crowdloan_fund_pot: Balance,
	para_id: ParaId,
	chain_id: u64,
) -> GenesisConfig {
	// This is supposed the be the simplest bytecode to revert without returning any data.
	// We will pre-deploy it under all of our precompiles to ensure they can be called from
	// within contracts. TODO We should have a test to ensure this is the right bytecode.
	// (PUSH1 0x00 PUSH1 0x00 REVERT)
	let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

	GenesisConfig {
		system: SystemConfig {
			code: WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 80))
				.collect(),
		},
		crowdloan_rewards: CrowdloanRewardsConfig {
			funded_amount: crowdloan_fund_pot,
		},
		sudo: SudoConfig { key: root_key },
		parachain_info: ParachainInfoConfig {
			parachain_id: para_id,
		},
		ethereum_chain_id: EthereumChainIdConfig { chain_id },
		evm: EVMConfig {
			// We need _some_ code inserted at the precompile address so that
			// the evm will actually call the address.
			accounts: Precompiles::used_addresses()
				.map(|addr| {
					(
						addr,
						GenesisAccount {
							nonce: Default::default(),
							balance: Default::default(),
							storage: Default::default(),
							code: revert_bytecode.clone(),
						},
					)
				})
				.collect(),
		},
		ethereum: EthereumConfig {},
		democracy: DemocracyConfig::default(),
		scheduler: SchedulerConfig {},
		parachain_staking: ParachainStakingConfig {
			candidates: candidates
				.iter()
				.cloned()
				.map(|(account, _, bond)| (account, bond))
				.collect(),
			nominations,
			inflation_config: moonbeam_inflation_config(),
		},
		council_collective: CouncilCollectiveConfig {
			phantom: Default::default(),
			members: council_members,
		},
		tech_comittee_collective: TechComitteeCollectiveConfig {
			phantom: Default::default(),
			members: vec![], // TODO : Set members
		},
		author_filter: AuthorFilterConfig {
			eligible_ratio: sp_runtime::Percent::from_percent(50),
		},
		author_mapping: AuthorMappingConfig {
			mappings: candidates
				.iter()
				.cloned()
				.map(|(account_id, author_id, _)| (author_id, account_id))
				.collect(),
		},
		treasury: Default::default(),
		migrations: Default::default(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_derived_pairs_1() {
		let mnemonic =
			"bottom drive obey lake curtain smoke basket hold race lonely fit walk".to_string();
		let accounts = 10;
		let pairs = derive_bip44_pairs_from_mnemonic::<ecdsa::Public>(&mnemonic, accounts);
		let first_account =
			get_account_id_from_pair::<ecdsa::Public>(pairs.first().unwrap().clone()).unwrap();
		let last_account =
			get_account_id_from_pair::<ecdsa::Public>(pairs.last().unwrap().clone()).unwrap();

		let expected_first_account =
			AccountId::from_str("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac").unwrap();
		let expected_last_account =
			AccountId::from_str("2898FE7a42Be376C8BC7AF536A940F7Fd5aDd423").unwrap();
		assert_eq!(first_account, expected_first_account);
		assert_eq!(last_account, expected_last_account);
		assert_eq!(pairs.len(), 10);
	}
	#[test]
	fn test_derived_pairs_2() {
		let mnemonic =
			"slab nerve salon plastic filter inherit valve ozone crash thumb quality whale"
				.to_string();
		let accounts = 20;
		let pairs = derive_bip44_pairs_from_mnemonic::<ecdsa::Public>(&mnemonic, accounts);
		let first_account =
			get_account_id_from_pair::<ecdsa::Public>(pairs.first().unwrap().clone()).unwrap();
		let last_account =
			get_account_id_from_pair::<ecdsa::Public>(pairs.last().unwrap().clone()).unwrap();

		let expected_first_account =
			AccountId::from_str("1e56ca71b596f2b784a27a2fdffef053dbdeff83").unwrap();
		let expected_last_account =
			AccountId::from_str("4148202BF0c0Ad7697Cff87EbB83340C80c947f8").unwrap();
		assert_eq!(first_account, expected_first_account);
		assert_eq!(last_account, expected_last_account);
		assert_eq!(pairs.len(), 20);
	}
}
