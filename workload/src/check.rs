use std::fmt::{Display, Formatter};

use enum_dispatch::enum_dispatch;

use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::types::Height;
use crate::utils::RetryConfig;

pub mod account_exist;
pub mod balance_shielded_source;
pub mod balance_shielded_target;
pub mod balance_source;
pub mod balance_target;
pub mod bond_decrease;
pub mod bond_increase;
pub mod reveal_pk;
mod utils;
pub mod validator_account;
pub mod validator_status;

#[enum_dispatch]
pub enum Check {
    RevealPk(reveal_pk::RevealPk),
    BalanceTarget(balance_target::BalanceTarget),
    BalanceSource(balance_source::BalanceSource),
    BalanceShieldedTarget(balance_shielded_target::BalanceShieldedTarget),
    BalanceShieldedSource(balance_shielded_source::BalanceShieldedSource),
    BondIncrease(bond_increase::BondIncrease),
    BondDecrease(bond_decrease::BondDecrease),
    AccountExist(account_exist::AccountExist),
    IsValidatorAccount(validator_account::ValidatorAccount),
    ValidatorStatus(validator_status::ValidatorStatus),
}

impl Display for Check {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

pub struct CheckInfo {
    pub execution_height: Height,
    pub check_height: Height,
}

#[enum_dispatch(Check)]
pub trait CheckContext {
    fn summary(&self) -> String;

    async fn do_check(
        &self,
        sdk: &Sdk,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), StepError>;
}
