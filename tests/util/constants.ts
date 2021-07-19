export const SPECS_PATH = `./moonbeam-test-specs`;

export const DEBUG_MODE = process.env.DEBUG_MODE || false;
export const DISPLAY_LOG = process.env.MOONBEAM_LOG || false;
export const MOONBEAM_LOG = process.env.MOONBEAM_LOG || "info";

export const BINARY_PATH = process.env.BINARY_PATH || `../target/release/moonbeam`;
export const RELAY_BINARY_PATH = process.env.RELAY_BINARY_PATH || `../target/release/polkadot`;
export const SPAWNING_TIME = 10000;

// Test variables
export const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000";
export const GENESIS_ACCOUNT = "0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b";
export const GENESIS_ACCOUNT_PRIVATE_KEY =
  "0x99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342";
export const TEST_ACCOUNT = "0x1111111111111111111111111111111111111111";
export const ALITH = "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac";
export const ALITH_PRIV_KEY = "0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133";
export const BALTATHAR = "0x3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0";
export const BALTATHAR_PRIV_KEY =
  "0x8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b";
export const CHARLETH = "0x798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc";
export const CHARLETH_PRIV_KEY =
  "0x0b6e18cafb6ed99687ec547bd28139cafdd2bffe70e6b688025de6b445aa5c5b";
export const DOROTHY = "0x773539d4Ac0e786233D90A233654ccEE26a613D9";
export const DOROTHY_PRIV_KEY =
  "0x39539ab1876910bbf3a223d84a29e28f1cb4e2e456503e7e91ed39b2e7223d68";
export const RANDOM_PRIV_KEY = "0x66d8d3bdfc9d678c1ea6dc3e15a81cb98dcd4d456f5ce0519479df1fba70cc5e";
export const ETHAN_PRIVKEY = "0x7dce9bc8babb68fec1409be38c8e1a52650206a7ed90ff956ae8a6d15eeaaef4";
export const ETHAN = "0xFf64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB";
export const RANDOM_ADDRESS = "0x39Cccb8cc2A821eB5cDADc656fF4229398AbA190";
export const GLMR = 1_000_000_000_000_000_000n;
export const DEFAULT_GENESIS_BALANCE = 2n ** 80n;
export const DEFAULT_GENESIS_STAKING = 1_000n * GLMR;
export const DEFAULT_GENESIS_MAPPING = 100n * GLMR;
export const MIN_GLMR_STAKING = 1000n * GLMR;
export const MIN_GLMR_NOMINATOR = 5n * GLMR;
export const GENESIS_ACCOUNT_BALANCE = DEFAULT_GENESIS_BALANCE;

// This is Alice
export const COLLATOR_ACCOUNT = "0xf24ff3a9cf04c71dbc94d0b566f7a27b94566cac";
export const COLLATOR_ACCOUNT_BALANCE =
  DEFAULT_GENESIS_BALANCE - DEFAULT_GENESIS_STAKING - DEFAULT_GENESIS_MAPPING;

// Current gas per second
export const GAS_PER_SECOND = 40_000_000;
// The real computation is 1_000_000_000_000 / 40_000_000, but we simplify to avoid bigint.
export const GAS_PER_WEIGHT = 1_000_000 / 40;

// Our weight limit is 500ms.
export const BLOCK_TX_LIMIT = GAS_PER_SECOND * 0.5;

// Current implementation is limiting block transactions to 75% of the block gas limit
export const BLOCK_TX_GAS_LIMIT = BLOCK_TX_LIMIT * 0.75;
// 125_000_000 Weight per extrinsics
export const EXTRINSIC_BASE_COST = 125_000_000 / GAS_PER_WEIGHT;

// Maximum extrinsic weight is taken from the max allowed transaction weight per block,
// minus the block initialization (10%) and minus the extrinsic base cost.
export const EXTRINSIC_GAS_LIMIT = BLOCK_TX_GAS_LIMIT - BLOCK_TX_LIMIT * 0.1 - EXTRINSIC_BASE_COST;
