use std::collections::HashMap;

use namada_sdk::{args, signing::SigningTxData, tx::Tx};

use crate::{
    check::Check, entities::Alias, executor::StepError, sdk::namada::Sdk, task::TaskSettings,
};

use super::tx_utils::merge_tx;
use super::{RetryConfig, Task, TaskContext};

#[derive(Clone)]
pub(super) struct Batch {
    tasks: Vec<Task>,
    settings: TaskSettings,
}

impl TaskContext for Batch {
    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let mut txs = vec![];
        for task in &self.tasks {
            let (tx, mut signing_data, _) = task.build_tx(sdk).await?;
            if signing_data.len() != 1 {
                return Err(StepError::Build("Unexpected sigining data".to_string()));
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
            let task_checks = task.build_checks(sdk, retry_config).await?;
            checks.extend(task_checks);
            match task {
                Task::NewWalletKeyPair(info) => {
                    reveal_pks.insert(info.source.clone(), source.to_owned());
                }
                Task::FaucetTransfer(target, amount, _task_settings) => {
                    balances
                        .entry(target.clone())
                        .and_modify(|balance| *balance += *amount as i64)
                        .or_insert(*amount as i64);
                }
                Task::TransparentTransfer(source, target, amount, _task_settings) => {
                    balances
                        .entry(target.clone())
                        .and_modify(|balance| *balance += *amount as i64)
                        .or_insert(*amount as i64);
                    balances
                        .entry(source.clone())
                        .and_modify(|balance| *balance -= *amount as i64)
                        .or_insert(-(*amount as i64));
                }
                Task::Bond(source, validator, amount, epoch, _task_settings) => {
                    bonds
                        .entry(format!("{}@{}", source.name, validator))
                        .and_modify(|(_epoch, bond_amount)| *bond_amount += *amount as i64)
                        .or_insert((*epoch, *amount as i64));
                    balances
                        .entry(source.clone())
                        .and_modify(|balance| *balance -= *amount as i64)
                        .or_insert(-(*amount as i64));
                }
                Task::Unbond(source, validator, amount, epoch, _task_settings) => {
                    bonds
                        .entry(format!("{}@{}", source.name, validator))
                        .and_modify(|(_epoch, bond_amount)| *bond_amount -= *amount as i64)
                        .or_insert((*epoch, -(*amount as i64)));
                }
                Task::Redelegate(source, from, to, amount, epoch, _task_settings) => {
                    bonds
                        .entry(format!("{}@{}", source.name, to))
                        .and_modify(|(_epoch, bond_amount)| *bond_amount += *amount as i64)
                        .or_insert((*epoch, *amount as i64));
                    bonds
                        .entry(format!("{}@{}", source.name, from))
                        .and_modify(|(_epoch, bond_amount)| *bond_amount -= *amount as i64)
                        .or_insert((*epoch, -(*amount as i64)));
                }
                Task::ShieldedTransfer(source, target, amount, _task_settings) => {
                    shielded_balances
                        .entry(source.clone())
                        .and_modify(|balance| *balance -= *amount as i64)
                        .or_insert(-(*amount as i64));
                    shielded_balances
                        .entry(target.clone())
                        .and_modify(|balance| *balance += *amount as i64)
                        .or_insert(*amount as i64);
                }
                Task::Shielding(source, target, amount, _task_settings) => {
                    balances
                        .entry(source.clone())
                        .and_modify(|balance| *balance -= *amount as i64)
                        .or_insert(-(*amount as i64));
                    shielded_balances
                        .entry(target.clone())
                        .and_modify(|balance| *balance += *amount as i64)
                        .or_insert(*amount as i64);
                }
                Task::Unshielding(source, target, amount, _task_settings) => {
                    balances
                        .entry(source.clone())
                        .and_modify(|balance| *balance += *amount as i64)
                        .or_insert(-(*amount as i64));
                    shielded_balances
                        .entry(target.clone())
                        .and_modify(|balance| *balance -= *amount as i64)
                        .or_insert(*amount as i64);
                }
                Task::ClaimRewards(_source, _validator, _task_settings) => {}
                _ => panic!(),
            };
        }

        let mut checks = vec![];
        let mut reveal_pks: HashMap<Alias, Alias> = HashMap::default();
        let mut balances: HashMap<Alias, i64> = HashMap::default();
        let mut shielded_balances: HashMap<Alias, i64> = HashMap::default();
        let mut bonds: HashMap<String, (u64, i64)> = HashMap::default();

        for (_, source) in reveal_pks {
            checks.push(Check::RevealPk(source));
        }

        for (alias, amount) in balances {
            let pre_balance = build_checks::utils::get_balance(sdk, &alias, retry_config).await?;
            if amount >= 0 {
                checks.push(Check::BalanceTarget(
                    alias,
                    pre_balance,
                    amount.unsigned_abs(),
                ));
            } else {
                checks.push(Check::BalanceSource(
                    alias,
                    pre_balance,
                    amount.unsigned_abs(),
                ));
            }
        }

        for (key, (epoch, amount)) in bonds {
            let (source, validator) = key.split_once('@').unwrap();
            let pre_bond = build_checks::utils::get_bond(
                sdk,
                &Alias::from(source),
                validator,
                epoch,
                retry_config,
            )
            .await?;
            if amount > 0 {
                checks.push(Check::BondIncrease(
                    Alias::from(source),
                    validator.to_owned(),
                    pre_bond,
                    amount.unsigned_abs(),
                ));
            } else {
                checks.push(Check::BondDecrease(
                    Alias::from(source),
                    validator.to_owned(),
                    pre_bond,
                    amount.unsigned_abs(),
                ));
            }
        }

        for (alias, amount) in shielded_balances {
            if let Some(pre_balance) =
                build_checks::utils::get_shielded_balance(sdk, &alias, None, true).await?
            {
                if amount >= 0 {
                    checks.push(Check::BalanceShieldedTarget(
                        alias,
                        pre_balance,
                        amount.unsigned_abs(),
                    ));
                } else {
                    checks.push(Check::BalanceShieldedSource(
                        alias,
                        pre_balance,
                        amount.unsigned_abs(),
                    ));
                }
            }
        }

        Ok(checks)
    }
}
