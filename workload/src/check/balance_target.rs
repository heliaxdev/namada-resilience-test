use std::collections::HashMap;

use namada_sdk::token;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::context::Ctx;
use crate::error::CheckError;
use crate::types::{Alias, Amount, Balance, Fee};
use crate::utils::{get_balance, is_native_denom, RetryConfig};

#[derive(TypedBuilder)]
pub struct BalanceTarget {
    target: Alias,
    pre_balance: Balance,
    denom: String,
    amount: Amount,
    #[builder(default)]
    allow_greater: bool,
}

impl BalanceTarget {
    pub fn target(&self) -> &Alias {
        &self.target
    }

    pub fn pre_balance(&self) -> Balance {
        self.pre_balance
    }

    pub fn denom(&self) -> &str {
        &self.denom
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }
}

impl CheckContext for BalanceTarget {
    fn summary(&self) -> String {
        format!("balance/target/'{}'/{}", self.denom, self.target.name)
    }

    async fn do_check(
        &self,
        ctx: &Ctx,
        fees: &HashMap<Alias, Fee>,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), CheckError> {
        let (target_address, post_balance) =
            get_balance(ctx, &self.target, &self.denom, retry_config).await?;

        let fee = if is_native_denom(&self.denom) {
            fees.get(&self.target).cloned().unwrap_or_default()
        } else {
            0u64
        };

        let check_balance = self
            .pre_balance
            .checked_add(token::Amount::from_u64(self.amount))
            .and_then(|b| b.checked_sub(token::Amount::from_u64(fee)))
            .ok_or_else(|| {
                CheckError::State(format!(
                    "BalanceTarget check error: {} balance is underflowing",
                    self.target.name
                ))
            })?;
        let is_expected =
            post_balance == check_balance || (self.allow_greater && post_balance > check_balance);

        let details = json!({
            "target_alias": self.target,
            "target": target_address.to_pretty_string(),
            "pre_balance": self.pre_balance,
            "amount": self.amount,
            "paid_fee": fee,
            "allow_greater": self.allow_greater,
            "post_balance": post_balance,
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height,
        });

        if is_expected {
            tracing::info!("Balance target increased: {details}");
            Ok(())
        } else {
            tracing::error!("Balance target is wrong: {details}");
            Err(CheckError::State(format!("BalanceTarget check error: post target amount is not equal to pre balance + amount: {} + {} - {fee} = {check_balance} != {post_balance}", self.pre_balance, self.amount)))
        }
    }
}
