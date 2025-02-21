// For retry
pub const MAX_RETRY_COUNT: u32 = 4;
pub const INIT_DELAY_SEC: u64 = 1;
pub const MAX_DELAY_SEC: u64 = 10;

// For batch
pub const MAX_BATCH_TX_NUM: u64 = 3;

pub const FAUCET_AMOUNT: u64 = 1_000_000;
pub const NATIVE_SCALE: u64 = namada_sdk::token::NATIVE_SCALE;
pub const DEFAULT_GAS_LIMIT: u64 = namada_sdk::DEFAULT_GAS_LIMIT * 2;
pub const DEFAULT_GAS_PRICE: f64 = 0.000001;
pub const DEFAULT_FEE_IN_NATIVE_TOKEN: u64 =
    ((DEFAULT_GAS_LIMIT as f64 * DEFAULT_GAS_PRICE) + 2.0) as u64;
pub const MIN_TRANSFER_BALANCE: u64 = 2 * NATIVE_SCALE;
pub const PROPOSAL_DEPOSIT: u64 = 50 * NATIVE_SCALE;
