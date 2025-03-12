// For retry
pub const MAX_RETRY_COUNT: u32 = 4;
pub const INIT_DELAY_SEC: u64 = 1;
pub const MAX_DELAY_SEC: u64 = 10;

// For batch
pub const MAX_BATCH_TX_NUM: u64 = 3;

// For bonding (They depend on the Namada parameters)
pub const PIPELINE_LEN: u64 = 2;
pub const UNBONDING_LEN: u64 = 3;

pub const FAUCET_AMOUNT: u64 = 1_000_000;
pub const NATIVE_SCALE: u64 = namada_sdk::token::NATIVE_SCALE;
pub const DEFAULT_GAS_LIMIT: u64 = namada_sdk::DEFAULT_GAS_LIMIT * 2;
pub const MIN_TRANSFER_BALANCE: u64 = MAX_BATCH_TX_NUM * NATIVE_SCALE + DEFAULT_GAS_LIMIT;
pub const PROPOSAL_DEPOSIT: u64 = 50 * NATIVE_SCALE;
