use namada_sdk::token;
use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::types::{Alias, Amount, Balance};
use crate::utils::{get_shielded_balance, RetryConfig};

#[derive(TypedBuilder)]
pub struct BalanceShieldedSource {
    target: Alias,
    pre_balance: Balance,
    amount: Amount,
}

impl BalanceShieldedSource {
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

impl CheckContext for BalanceShieldedSource {
    fn summary(&self) -> String {
        format!("balance-shielded/source/{}", self.target.name)
    }

    async fn do_check(
        &self,
        sdk: &Sdk,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), StepError> {
        let post_balance = get_shielded_balance(
            sdk,
            &self.target,
            Some(check_info.execution_height),
            true,
            retry_config,
        )
        .await?
        .ok_or_else(|| {
            antithesis_sdk::assert_unreachable!(
                "BalanceShieldedSource target doesn't exist.",
                &json!({
                    "source_alias": self.target,
                    "pre_balance": self.pre_balance,
                    "amount": self.amount,
                    "execution_height": check_info.execution_height,
                    "check_height": check_info.check_height,
                })
            );
            StepError::StateCheck(format!(
                "BalanceShieldedSource check error: {} balance doesn't exist",
                self.target.name
            ))
        })?;

        let check_balance = self
            .pre_balance
            .checked_sub(token::Amount::from_u64(self.amount))
            .ok_or_else(|| {
                StepError::StateCheck(format!(
                    "BalanceShieldedSource check error: {} balance is underflowing",
                    self.target.name
                ))
            })?;

        let details = json!({
            "source_alias": self.target,
            "pre_balance": self.pre_balance,
            "amount": self.amount,
            "post_balance": post_balance,
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height
        });

        antithesis_sdk::assert_always!(
            post_balance.eq(&check_balance),
            "BalanceShielded source decreased",
            &details
        );

        if post_balance.eq(&check_balance) {
            Ok(())
        } else {
            tracing::error!("{}", details);
            Err(StepError::StateCheck(format!("BalanceShieldedSource check error: post source amount is not equal to pre balance - amount: {} - {} = {check_balance} != {post_balance}", self.pre_balance, self.amount)))
        }
    }
}
