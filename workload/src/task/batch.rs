use std::collections::HashMap;

use namada_sdk::{args, signing::SigningTxData, tx::Tx};
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{Task, TaskContext, TaskSettings};
use crate::types::Alias;
use crate::utils::{get_balance, get_bond, get_shielded_balance, merge_tx, RetryConfig};

#[derive(Clone, Debug, TypedBuilder)]
pub struct Batch {
    tasks: Vec<Task>,
    settings: TaskSettings,
}

impl TaskContext for Batch {
    fn name(&self) -> String {
        "batch".to_string()
    }

    fn summary(&self) -> String {
        let tasks = self
            .tasks
            .iter()
            .map(|task| task.to_string())
            .collect::<Vec<String>>();
        format!("batch-{} -> {}", tasks.len(), tasks.join(" -> "))
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let mut txs = vec![];
        for task in &self.tasks {
            let (tx, mut signing_data, _) = Box::pin(task.build_tx(sdk)).await?;
            if signing_data.len() != 1 {
                return Err(StepError::BuildTx("Unexpected sigining data".to_string()));
            }
            txs.push((tx, signing_data.remove(0)));
        }

        merge_tx(sdk, txs, &self.settings).await
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let mut checks = vec![];
        for task in &self.tasks {
            let task_checks = Box::pin(task.build_checks(sdk, retry_config)).await?;
            checks.extend(task_checks);
        }

        let mut prepared_checks = vec![];
        let mut balances: HashMap<Alias, i64> = HashMap::default();
        let mut shielded_balances: HashMap<Alias, i64> = HashMap::default();
        let mut bonds: HashMap<String, (u64, i64)> = HashMap::default();
        for check in checks {
            match check {
                Check::RevealPk(_) => prepared_checks.push(check),
                Check::BalanceSource(balance_source) => {
                    balances
                        .entry(balance_source.target().clone())
                        .and_modify(|balance| *balance -= balance_source.amount() as i64)
                        .or_insert(-(balance_source.amount() as i64));
                }
                Check::BalanceTarget(balance_target) => {
                    balances
                        .entry(balance_target.target().clone())
                        .and_modify(|balance| *balance += balance_target.amount() as i64)
                        .or_insert(balance_target.amount() as i64);
                }
                Check::BalanceShieldedSource(balance_source) => {
                    shielded_balances
                        .entry(balance_source.target().clone())
                        .and_modify(|balance| *balance -= balance_source.amount() as i64)
                        .or_insert(-(balance_source.amount() as i64));
                }
                Check::BalanceShieldedTarget(balance_target) => {
                    shielded_balances
                        .entry(balance_target.target().clone())
                        .and_modify(|balance| *balance += balance_target.amount() as i64)
                        .or_insert(balance_target.amount() as i64);
                }
                Check::BondIncrease(bond_increase) => {
                    bonds
                        .entry(format!(
                            "{}@{}",
                            bond_increase.target().name,
                            bond_increase.validator()
                        ))
                        .and_modify(|(_epoch, bond_amount)| {
                            *bond_amount += bond_increase.amount() as i64
                        })
                        .or_insert((bond_increase.epoch(), bond_increase.amount() as i64));
                    balances
                        .entry(bond_increase.target().clone())
                        .and_modify(|balance| *balance -= bond_increase.amount() as i64)
                        .or_insert(-(bond_increase.amount() as i64));
                }
                Check::BondDecrease(bond_decrease) => {
                    bonds
                        .entry(format!(
                            "{}@{}",
                            bond_decrease.target().name,
                            bond_decrease.validator()
                        ))
                        .and_modify(|(_epoch, bond_amount)| {
                            *bond_amount -= bond_decrease.amount() as i64
                        })
                        .or_insert((bond_decrease.epoch(), bond_decrease.amount() as i64));
                    balances
                        .entry(bond_decrease.target().clone())
                        .and_modify(|balance| *balance += bond_decrease.amount() as i64)
                        .or_insert(-(bond_decrease.amount() as i64));
                }
                _ => {
                    return Err(StepError::BuildCheck(format!(
                        "Unexpected check happened: {check}"
                    )))
                }
            }
        }

        for (alias, amount) in balances {
            let (_, pre_balance) = get_balance(sdk, &alias, retry_config).await?;
            if amount >= 0 {
                prepared_checks.push(Check::BalanceTarget(
                    check::balance_target::BalanceTarget::builder()
                        .target(alias)
                        .pre_balance(pre_balance)
                        .amount(amount.unsigned_abs())
                        .build(),
                ));
            } else {
                prepared_checks.push(Check::BalanceSource(
                    check::balance_source::BalanceSource::builder()
                        .target(alias)
                        .pre_balance(pre_balance)
                        .amount(amount.unsigned_abs())
                        .build(),
                ));
            }
        }

        for (key, (epoch, amount)) in bonds {
            let (source, validator) = key.split_once('@').unwrap();
            let pre_bond =
                get_bond(sdk, &Alias::from(source), validator, epoch, retry_config).await?;
            if amount > 0 {
                prepared_checks.push(Check::BondIncrease(
                    check::bond_increase::BondIncrease::builder()
                        .target(Alias::from(source))
                        .validator(validator.to_owned())
                        .pre_bond(pre_bond)
                        .epoch(epoch)
                        .amount(amount.unsigned_abs())
                        .build(),
                ));
            } else {
                prepared_checks.push(Check::BondDecrease(
                    check::bond_decrease::BondDecrease::builder()
                        .target(Alias::from(source))
                        .validator(validator.to_owned())
                        .pre_bond(pre_bond)
                        .epoch(epoch)
                        .amount(amount.unsigned_abs())
                        .build(),
                ));
            }
        }

        for (alias, amount) in shielded_balances {
            if let Some(pre_balance) = get_shielded_balance(sdk, &alias, None, true).await? {
                if amount >= 0 {
                    prepared_checks.push(Check::BalanceShieldedTarget(
                        check::balance_shielded_target::BalanceShieldedTarget::builder()
                            .target(alias)
                            .pre_balance(pre_balance)
                            .amount(amount.unsigned_abs())
                            .build(),
                    ));
                } else {
                    prepared_checks.push(Check::BalanceShieldedSource(
                        check::balance_shielded_source::BalanceShieldedSource::builder()
                            .target(alias)
                            .pre_balance(pre_balance)
                            .amount(amount.unsigned_abs())
                            .build(),
                    ));
                }
            }
        }

        Ok(prepared_checks)
    }

    fn update_state(&self, state: &mut State, _with_fee: bool) {
        state.modify_balance_fee(&self.settings.gas_payer, self.settings.gas_limit);
        for task in &self.tasks {
            task.update_state(state, false);
        }
    }
}
