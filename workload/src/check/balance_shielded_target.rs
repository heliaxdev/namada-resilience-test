use namada_sdk::token;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::types::{Alias, Amount, Balance};
use crate::utils::{get_shielded_balance, shielded_sync_with_retry, RetryConfig};

#[derive(TypedBuilder)]
pub struct BalanceShieldedTarget {
    target: Alias,
    pre_balance: Balance,
    amount: Amount,
}

impl BalanceShieldedTarget {
    pub fn target(&self) -> &Alias {
        &self.target
    }

    pub fn pre_balance(&self) -> Balance {
        self.pre_balance
    }

    pub fn amount(&self) -> Amount {
        self.amount
    }
}

impl CheckContext for BalanceShieldedTarget {
    fn summary(&self) -> String {
        format!("balance-shielded/target/{}", self.target.name)
    }

    async fn do_check(
        &self,
        sdk: &Sdk,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), StepError> {
        shielded_sync_with_retry(sdk, &self.target, Some(check_info.execution_height), true)
            .await?;

        let post_balance = get_shielded_balance(sdk, &self.target, retry_config)
            .await?
            .ok_or_else(|| {
                antithesis_sdk::assert_unreachable!(
                    "BalanceShieldedTarget target doesn't exist.",
                    &json!({
                        "target_alias": self.target,
                        "pre_balance": self.pre_balance,
                        "amount": self.amount,
                        "execution_height": check_info.execution_height,
                        "check_height": check_info.check_height,
                    })
                );
                StepError::StateCheck(format!(
                    "BalanceShieldedTarget check error: {} balance doesn't exist",
                    self.target.name
                ))
            })?;

        let check_balance = self
            .pre_balance
            .checked_add(token::Amount::from_u64(self.amount))
            .ok_or_else(|| {
                StepError::StateCheck(format!(
                    "BalanceShieldedTarget check error: {} balance is overflowing",
                    self.target.name
                ))
            })?;

        let details = json!({
            "target_alias": self.target,
            "pre_balance": self.pre_balance,
            "amount": self.amount,
            "post_balance": post_balance,
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height
        });

        antithesis_sdk::assert_always!(
            post_balance.eq(&check_balance),
            "BalanceShielded target increased",
            &details
        );

        if post_balance.eq(&check_balance) {
            Ok(())
        } else {
            tracing::error!("{}", details);
            Err(StepError::StateCheck(format!("BalanceShieldedTarget check error: post target amount is not equal to pre balance + amount: {} + {} = {check_balance} != {post_balance}", self.pre_balance, self.amount)))
        }
    }
}
