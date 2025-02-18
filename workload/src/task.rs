use std::{
    collections::{BTreeSet, HashMap},
    fmt::Display,
    time::Duration,
};

use namada_sdk::dec::Dec;
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{
    build_checks,
    check::Check,
    constants::DEFAULT_GAS_LIMIT,
    entities::Alias,
    execute::{
        batch::execute_tx_batch,
        become_validator::execute_tx_become_validator,
        bond::{build_tx_bond, execute_tx_bond},
        change_consensus_keys::execute_tx_change_consensus_key,
        change_metadata::execute_tx_change_metadata,
        claim_rewards::{build_tx_claim_rewards, execute_tx_claim_rewards},
        deactivate_validator::execute_tx_deactivate_validator,
        default_proposal::execute_tx_default_proposal,
        faucet_transfer::execute_faucet_transfer,
        init_account::execute_tx_init_account,
        new_wallet_keypair::execute_new_wallet_key_pair,
        reactivate_validator::execute_tx_reactivate_validator,
        redelegate::{build_tx_redelegate, execute_tx_redelegate},
        reveal_pk::execute_reveal_pk,
        shielded::{build_tx_shielded_transfer, execute_tx_shielded_transfer},
        shielding::{build_tx_shielding, execute_tx_shielding},
        transparent_transfer::{build_tx_transparent_transfer, execute_tx_transparent_transfer},
        unbond::{build_tx_unbond, execute_tx_unbond},
        unshielding::execute_tx_unshielding,
        update_account::execute_tx_update_account,
        vote::execute_tx_vote,
    },
    executor::StepError,
    sdk::namada::Sdk,
};

#[derive(Clone, Debug)]
pub struct TaskSettings {
    pub signers: BTreeSet<Alias>,
    pub gas_payer: Alias,
    pub gas_limit: u64,
}

impl TaskSettings {
    pub fn new(signers: BTreeSet<Alias>, gas_payer: Alias) -> Self {
        Self {
            signers,
            gas_payer,
            gas_limit: DEFAULT_GAS_LIMIT * 2,
        }
    }

    pub fn faucet() -> Self {
        Self {
            signers: BTreeSet::from_iter(vec![Alias::faucet()]),
            gas_payer: Alias::faucet(),
            gas_limit: DEFAULT_GAS_LIMIT * 2,
        }
    }

    pub fn faucet_batch(size: usize) -> Self {
        Self {
            signers: BTreeSet::from_iter(vec![Alias::faucet()]),
            gas_payer: Alias::faucet(),
            gas_limit: DEFAULT_GAS_LIMIT * size as u64 * 2,
        }
    }
}

pub type Target = Alias;
pub type Source = Alias;
pub type PaymentAddress = Alias;
pub type Amount = u64;
pub type Address = String;
pub type Epoch = u64;
pub type Threshold = u64;
pub type WalletAlias = Alias;
pub type CommissionRate = Dec;
pub type CommissionChange = Dec;
pub type ProposalId = u64;
pub type Vote = String;

#[derive(Clone, Debug)]
pub enum Task {
    NewWalletKeyPair(Source),
    FaucetTransfer(Target, Amount, TaskSettings),
    TransparentTransfer(Source, Target, Amount, TaskSettings),
    Bond(Source, Address, Amount, Epoch, TaskSettings),
    Unbond(Source, Address, Amount, Epoch, TaskSettings),
    Redelegate(Source, Address, Address, Amount, Epoch, TaskSettings),
    ClaimRewards(Source, Address, TaskSettings),
    Batch(Vec<Task>, TaskSettings),
    ShieldedTransfer(Source, Target, Amount, TaskSettings),
    Shielding(Source, PaymentAddress, Amount, TaskSettings),
    InitAccount(Source, BTreeSet<Source>, Threshold, TaskSettings),
    Unshielding(PaymentAddress, Target, Amount, TaskSettings),
    BecomeValidator(
        Source,
        WalletAlias,
        WalletAlias,
        WalletAlias,
        WalletAlias,
        CommissionRate,
        CommissionChange,
        TaskSettings,
    ),
    ChangeMetadata(Source, String, String, String, String, String, TaskSettings),
    ChangeConsensusKeys(Source, Alias, TaskSettings),
    DeactivateValidator(Source, TaskSettings),
    ReactivateValidator(Source, TaskSettings),
    UpdateAccount(Source, BTreeSet<Source>, Threshold, TaskSettings),
    DefaultProposal(Source, Epoch, Epoch, Epoch, TaskSettings),
    Vote(Source, ProposalId, Vote, TaskSettings),
}

impl Task {
    pub fn raw_type(&self) -> String {
        match self {
            Task::NewWalletKeyPair(_) => "new-wallet-keypair".to_string(),
            Task::FaucetTransfer(_, _, _) => "faucet-transfer".to_string(),
            Task::TransparentTransfer(_, _, _, _) => "transparent-transfer".to_string(),
            Task::Bond(_, _, _, _, _) => "bond".to_string(),
            Task::Unbond(_, _, _, _, _) => "unbond".to_string(),
            Task::Redelegate(_, _, _, _, _, _) => "relegate".to_string(),
            Task::ClaimRewards(_, _, _) => "claim-rewards".to_string(),
            Task::Batch(_, _) => "batch".to_string(),
            Task::Shielding(_, _, _, _) => "shielding".to_string(),
            Task::Unshielding(_, _, _, _) => "unshielding".to_string(),
            Task::ShieldedTransfer(_, _, _, _) => "shielded-transfer".to_string(),
            Task::InitAccount(_, _, _, _) => "init-account".to_string(),
            Task::BecomeValidator(_, _, _, _, _, _, _, _) => "become-validator".to_string(),
            Task::ChangeMetadata(_, _, _, _, _, _, _) => "change-metadata".to_string(),
            Task::ChangeConsensusKeys(_, _, _) => "change-consensus-keys".to_string(),
            Task::UpdateAccount(_, _, _, _) => "update-account".to_string(),
            Task::DeactivateValidator(_, _) => "deactivate-validator".to_string(),
            Task::ReactivateValidator(_, _) => "reactivate-validator".to_string(),
            Task::DefaultProposal(_, _, _, _, _) => "default-proposal".to_string(),
            Task::Vote(_, _, _, _) => "vote-proposal".to_string(),
        }
    }

    pub fn task_settings(&self) -> Option<&TaskSettings> {
        match self {
            Task::NewWalletKeyPair(_alias) => None,
            Task::FaucetTransfer(_alias, _, task_settings) => Some(task_settings),
            Task::TransparentTransfer(_alias, _alias1, _, task_settings) => Some(task_settings),
            Task::Bond(_alias, _, _, _, task_settings) => Some(task_settings),
            Task::Unbond(_alias, _, _, _, task_settings) => Some(task_settings),
            Task::Redelegate(_alias, _, _, _, _, task_settings) => Some(task_settings),
            Task::ClaimRewards(_alias, _, task_settings) => Some(task_settings),
            Task::Batch(_tasks, task_settings) => Some(task_settings),
            Task::Shielding(_alias, _alias1, _, task_settings) => Some(task_settings),
            Task::InitAccount(_alias, _btree_set, _, task_settings) => Some(task_settings),
            Task::BecomeValidator(_, _, _, _, _, _, _, task_settings) => Some(task_settings),
            Task::ShieldedTransfer(_, _, _, task_settings) => Some(task_settings),
            Task::Unshielding(_, _, _, task_settings) => Some(task_settings),
            Task::ChangeMetadata(_, _, _, _, _, _, task_settings) => Some(task_settings),
            Task::ChangeConsensusKeys(_, _, task_settings) => Some(task_settings),
            Task::UpdateAccount(_alias, _, _, task_settings) => Some(task_settings),
            Task::DeactivateValidator(_, task_settings) => Some(task_settings),
            Task::ReactivateValidator(_, task_settings) => Some(task_settings),
            Task::DefaultProposal(_, _, _, _, task_settings) => Some(task_settings),
            Task::Vote(_, _, _, task_settings) => Some(task_settings),
        }
    }

    pub async fn build_check(&self, sdk: &Sdk) -> Result<Vec<Check>, StepError> {
        let retry_config = Self::retry_config();
        match self {
            Task::NewWalletKeyPair(source) => Ok(vec![Check::RevealPk(source.clone())]),
            Task::FaucetTransfer(target, amount, _) => {
                build_checks::faucet::faucet(sdk, target, *amount, retry_config).await
            }
            Task::TransparentTransfer(source, target, amount, _) => {
                build_checks::transparent_transfer::transparent_transfer(
                    sdk,
                    source,
                    target,
                    *amount,
                    retry_config,
                )
                .await
            }
            Task::Bond(source, validator, amount, epoch, _) => {
                build_checks::bond::bond(sdk, source, validator, *amount, *epoch, retry_config)
                    .await
            }
            Task::InitAccount(alias, sources, threshold, _) => {
                Ok(build_checks::init_account::init_account(
                    alias,
                    sources,
                    *threshold,
                )
                .await)
            }
            Task::Redelegate(source, from, to, amount, epoch, _) => {
                build_checks::redelegate::redelegate(
                    sdk,
                    source,
                    from,
                    to,
                    *amount,
                    *epoch,
                    retry_config,
                )
                .await
            }
            Task::Unbond(source, validator, amount, epoch, _) => {
                build_checks::unbond::unbond(
                    sdk,
                    source,
                    validator,
                    *amount,
                    *epoch,
                    retry_config,
                )
                .await
            }
            Task::ClaimRewards(_source, _validator, _) => Ok(vec![]),
            Task::ShieldedTransfer(source, target, amount, _) => {
                build_checks::shielded_transfer::shielded_transfer(
                    sdk,
                    source,
                    target,
                    *amount,
                    false,
                )
                .await
            }
            Task::Shielding(source, target, amount, _) => {
                build_checks::shielding::shielding(
                    sdk,
                    source,
                    target,
                    *amount,
                    false,
                    retry_config,
                )
                .await
            }
            Task::Unshielding(source, target, amount, _) => {
                build_checks::unshielding::unshielding(
                    sdk,
                    source,
                    target,
                    *amount,
                    false,
                    retry_config,
                )
                .await
            }
            Task::BecomeValidator(source, _, _, _, _, _, _, _) => {
                Ok(build_checks::become_validator::become_validator(source).await)
            }
            Task::ChangeMetadata(_, _, _, _, _, _, _) => {
                Ok(vec![])
            }
            Task::ChangeConsensusKeys(_, _, _) => {
                Ok(vec![])
            }
            Task::UpdateAccount(target, sources, threshold, _) => {
                Ok(build_checks::update_account::update_account_build_checks(
                    target,
                    sources,
                    *threshold,
                )
                .await)
            }
            Task::DeactivateValidator(target, _) => {
                Ok(build_checks::deactivate_validator::deactivate_validator_build_checks(
                    target,
                )
                .await)
            }
            Task::ReactivateValidator(target, _) => {
                Ok(build_checks::reactivate_validator::reactivate_validator_build_checks(
                    target,
                )
                .await)
            }
            Task::DefaultProposal(source, _start_epoch, _end_epoch, _grace_epoch, _) => {
                build_checks::proposal::proposal(sdk, source, retry_config).await
            }
            Task::Vote(_, _, _, _) => Ok(vec![]),
            Task::Batch(tasks, _) => {
                let mut checks = vec![];

                let mut reveal_pks: HashMap<Alias, Alias> = HashMap::default();
                let mut balances: HashMap<Alias, i64> = HashMap::default();
                let mut shielded_balances: HashMap<Alias, i64> = HashMap::default();
                let mut bonds: HashMap<String, (u64, i64)> = HashMap::default();

                for task in tasks {
                    match &task {
                        Task::NewWalletKeyPair(source) => {
                            reveal_pks.insert(source.clone(), source.to_owned());
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
                    if let Some(pre_balance) = build_checks::utils::get_shielded_balance(
                        sdk,
                        &alias,
                        None,
                        true,
                    )
                    .await?
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
    }

    pub async fn execute(&self, sdk: &Sdk) -> Result<Option<u64>, StepError> {
        match self {
            Task::NewWalletKeyPair(alias) => {
                let public_key = execute_new_wallet_key_pair(sdk, alias).await?;
                execute_reveal_pk(sdk, public_key).await
            }
            Task::FaucetTransfer(target, amount, settings) => {
                execute_faucet_transfer(sdk, target, *amount, settings).await
            }
            Task::TransparentTransfer(source, target, amount, settings) => {
                execute_tx_transparent_transfer(sdk, source, target, *amount, settings).await
            }
            Task::Bond(source, validator, amount, _epoch, settings) => {
                execute_tx_bond(sdk, source, validator, *amount, settings).await
            }
            Task::InitAccount(source, sources, threshold, settings) => {
                execute_tx_init_account(sdk, source, sources, *threshold, settings).await
            }
            Task::Redelegate(source, from, to, amount, _epoch, settings) => {
                execute_tx_redelegate(sdk, source, from, to, *amount, settings).await
            }
            Task::Unbond(source, validator, amount, _epoch, settings) => {
                execute_tx_unbond(sdk, source, validator, *amount, settings).await
            }
            Task::ClaimRewards(source, validator, settings) => {
                execute_tx_claim_rewards(sdk, source, validator, settings).await
            }
            Task::ShieldedTransfer(source, target, amount, settings) => {
                execute_tx_shielded_transfer(sdk, source, target, *amount, settings).await
            }
            Task::Shielding(source, target, amount, settings) => {
                execute_tx_shielding(sdk, source, target, *amount, settings).await
            }
            Task::Unshielding(source, target, amount, settings) => {
                execute_tx_unshielding(sdk, source, target, *amount, settings).await
            }
            Task::DeactivateValidator(target, settings) => {
                execute_tx_deactivate_validator(sdk, target, settings).await
            }
            Task::ReactivateValidator(target, settings) => {
                execute_tx_reactivate_validator(sdk, target, settings).await
            }
            Task::Vote(source, proposal_id, vote, settings) => {
                execute_tx_vote(sdk, source, *proposal_id, vote, settings).await
            }
            Task::ChangeMetadata(
                source,
                website,
                email,
                discord,
                description,
                avatar,
                settings,
            ) => {
                execute_tx_change_metadata(
                    sdk,
                    source,
                    website,
                    email,
                    discord,
                    description,
                    avatar,
                    settings,
                )
                .await
            }
            Task::ChangeConsensusKeys(source, alias, settings) => {
                execute_tx_change_consensus_key(sdk, source, alias, settings).await
            }
            Task::UpdateAccount(target, sources, threshold, settings) => {
                execute_tx_update_account(sdk, target, sources, *threshold, settings).await
            }
            Task::BecomeValidator(alias, t, t1, t2, t3, comm, max_comm_change, settings) => {
                execute_tx_become_validator(
                    sdk,
                    alias,
                    t,
                    t1,
                    t2,
                    t3,
                    *comm,
                    *max_comm_change,
                    settings,
                )
                .await
            }
            Task::DefaultProposal(source, start_epoch, end_epoch, grace_epoch, settings) => {
                execute_tx_default_proposal(
                    sdk,
                    source,
                    *start_epoch,
                    *end_epoch,
                    *grace_epoch,
                    settings,
                )
                .await
            }
            Task::Batch(tasks, task_settings) => {
                let mut txs = vec![];
                for task in tasks {
                    let (tx, signing_data, _) = match task {
                        Task::TransparentTransfer(source, target, amount, settings) => {
                            build_tx_transparent_transfer(sdk, source, target, *amount, settings)
                                .await?
                        }
                        Task::Bond(source, validator, amount, _epoch, settings) => {
                            build_tx_bond(sdk, source, validator, *amount, settings).await?
                        }
                        Task::Redelegate(source, from, to, amount, _epoch, task_settings) => {
                            build_tx_redelegate(sdk, source, from, to, *amount, task_settings)
                                .await?
                        }
                        Task::Unbond(source, validator, amount, _epoch, settings) => {
                            build_tx_unbond(sdk, source, validator, *amount, settings).await?
                        }
                        Task::ShieldedTransfer(source, target, amount, settings) => {
                            build_tx_shielded_transfer(sdk, source, target, *amount, settings)
                                .await?
                        }
                        Task::Shielding(source, target, amount, settings) => {
                            build_tx_shielding(sdk, source, target, *amount, settings).await?
                        }
                        Task::ClaimRewards(source, validator, settings) => {
                            build_tx_claim_rewards(sdk, source, validator, settings).await?
                        }
                        _ => panic!(),
                    };
                    txs.push((tx, signing_data));
                }

                execute_tx_batch(sdk, txs, task_settings).await
            }
        }
    }

    fn retry_config() -> RetryFutureConfig<ExponentialBackoff, NoOnRetry> {
        RetryFutureConfig::new(4)
            .exponential_backoff(Duration::from_secs(1))
            .max_delay(Duration::from_secs(10))
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Task::NewWalletKeyPair(source) => write!(f, "wallet-key-pair/{}", source.name),
            Task::FaucetTransfer(target, amount, _) => {
                write!(f, "faucet-transfer/{}/{}", target.name, amount)
            }
            Task::TransparentTransfer(source, target, amount, _) => {
                write!(
                    f,
                    "transparent-transfer/{}/{}/{}",
                    source.name, target.name, amount
                )
            }
            Task::Bond(source, validator, amount, _, _) => {
                write!(f, "bond/{}/{}/{}", source.name, validator, amount)
            }
            Task::Unbond(source, validator, amount, _, _) => {
                write!(f, "unbond/{}/{}/{}", source.name, validator, amount)
            }
            Task::ShieldedTransfer(source, target, amount, _) => {
                write!(
                    f,
                    "shielded-transfer/{}/{}/{}",
                    source.name, target.name, amount
                )
            }
            Task::Shielding(source, target, amount, _) => {
                write!(f, "shielding/{}/{}/{}", source.name, target.name, amount)
            }
            Task::Unshielding(source, target, amount, _) => {
                write!(f, "unshielding/{}/{}/{}", source.name, target.name, amount)
            }
            Task::InitAccount(alias, _, threshold, _) => {
                write!(f, "init-account/{}/{}", alias.name, threshold)
            }
            Task::BecomeValidator(alias, _, _, _, _, _, _, _) => {
                write!(f, "become-validator/{}", alias.name)
            }
            Task::ClaimRewards(alias, validator, _) => {
                write!(f, "claim-rewards/{}/{}", alias.name, validator)
            }
            Task::Redelegate(source, from, to, amount, _, _) => {
                write!(f, "redelegate/{}/{}/{}/{}", source.name, from, to, amount)
            }
            Task::ChangeMetadata(alias, _, _, _, _, _, _) => {
                write!(f, "change-metadata/{}", alias.name)
            }
            Task::ChangeConsensusKeys(alias, _, _) => {
                write!(f, "change-consensus-keys/{}", alias.name)
            }
            Task::UpdateAccount(source, _, _, _) => {
                write!(f, "update-account/{}", source.name)
            }
            Task::DeactivateValidator(source, _) => {
                write!(f, "deactivate-validator/{}", source.name)
            }
            Task::ReactivateValidator(source, _) => {
                write!(f, "reactivate-validator/{}", source.name)
            }
            Task::DefaultProposal(source, _, _, _, _) => {
                write!(f, "default-proposal/{}", source.name)
            }
            Task::Vote(source, proposal_id, vote, _) => {
                write!(f, "vote-proposal/{}/{}/{}", source.name, proposal_id, vote)
            }
            Task::Batch(tasks, _) => {
                let tasks = tasks
                    .iter()
                    .map(|task| task.to_string())
                    .collect::<Vec<String>>();
                write!(f, "batch-{} -> {}", tasks.len(), tasks.join(" -> "))
            }
        }
    }
}
