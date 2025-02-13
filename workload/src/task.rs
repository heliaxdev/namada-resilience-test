use std::{collections::BTreeSet, fmt::Display};

use namada_sdk::dec::Dec;

use crate::{
    constants::DEFAULT_GAS_LIMIT,
    entities::Alias,
    execute::{
        batch::execute_tx_batch,
        become_validator::build_tx_become_validator,
        bond::{build_tx_bond, execute_tx_bond},
        change_consensus_keys::{build_tx_change_consensus_key, execute_tx_change_consensus_key},
        change_metadata::build_tx_change_metadata,
        claim_rewards::{build_tx_claim_rewards, execute_tx_claim_rewards},
        deactivate_validator::{build_tx_deactivate_validator, execute_tx_deactivate_validator},
        default_proposal::{build_tx_default_proposal, execute_tx_default_proposal},
        faucet_transfer::execute_faucet_transfer,
        init_account::{build_tx_init_account, execute_tx_init_account},
        new_wallet_keypair::execute_new_wallet_key_pair,
        reactivate_validator::{build_tx_reactivate_validator, execute_tx_reactivate_validator},
        redelegate::{build_tx_redelegate, execute_tx_redelegate},
        reveal_pk::execute_reveal_pk,
        shielded::{build_tx_shielded_transfer, execute_tx_shielded_transfer},
        shielding::{build_tx_shielding, execute_tx_shielding},
        transparent_transfer::execute_tx_transparent_transfer,
        unbond::{build_tx_unbond, execute_tx_unbond},
        unshielding::{build_tx_unshielding, execute_tx_unshielding},
        update_account::{build_tx_update_account, execute_tx_update_account},
        vote::{build_tx_vote, execute_tx_vote},
    },
    sdk::namada::Sdk,
    steps::StepError,
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
                let (mut tx, signing_data, tx_args) =
                    build_tx_deactivate_validator(sdk, target, settings).await?;
                execute_tx_deactivate_validator(sdk, &mut tx, signing_data, &tx_args).await?
            }
            Task::ReactivateValidator(target, settings) => {
                let (mut tx, signing_data, tx_args) =
                    build_tx_reactivate_validator(sdk, target, settings).await?;
                execute_tx_reactivate_validator(sdk, &mut tx, signing_data, &tx_args).await?
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
                let (mut tx, signing_data, tx_args) = build_tx_change_metadata(
                    sdk,
                    source,
                    website,
                    email,
                    discord,
                    description,
                    avatar,
                    settings,
                )
                .await?;
                execute_tx_shielding(sdk, &mut tx, signing_data, &tx_args).await?
            }
            Task::ChangeConsensusKeys(source, alias, settings) => {
                let (mut tx, signing_data, tx_args) =
                    build_tx_change_consensus_key(sdk, source, alias, settings).await?;
                execute_tx_change_consensus_key(sdk, &mut tx, signing_data, &tx_args).await?
            }
            Task::UpdateAccount(target, sources, threshold, settings) => {
                let (mut tx, signing_data, tx_args) =
                    build_tx_update_account(sdk, target, sources, threshold, settings).await?;
                execute_tx_update_account(sdk, &mut tx, signing_data, &tx_args).await?
            }
            Task::BecomeValidator(alias, t, t1, t2, t3, comm, max_comm_change, settings) => {
                let (mut tx, signing_data, tx_args) = build_tx_become_validator(
                    sdk,
                    alias,
                    t,
                    t1,
                    t2,
                    t3,
                    comm,
                    max_comm_change,
                    settings,
                )
                .await?;
                execute_tx_shielding(sdk, &mut tx, signing_data, &tx_args).await?
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
                            build_tx_transparent_transfer(sdk, source, target, amount, settings)
                                .await?
                        }
                        Task::Bond(source, validator, amount, _epoch, settings) => {
                            build_tx_bond(sdk, source, validator, amount, settings).await?
                        }
                        Task::Redelegate(source, from, to, amount, _epoch, task_settings) => {
                            build_tx_redelegate(sdk, source, from, to, amount, task_settings)
                                .await?
                        }
                        Task::Unbond(source, validator, amount, _epoch, settings) => {
                            build_tx_unbond(sdk, source, validator, amount, settings).await?
                        }
                        Task::ShieldedTransfer(source, target, amount, settings) => {
                            build_tx_shielded_transfer(sdk, source, target, amount, settings)
                                .await?
                        }
                        Task::Shielding(source, target, amount, settings) => {
                            build_tx_shielding(sdk, source, target, amount, settings).await?
                        }
                        Task::ClaimRewards(source, validator, settings) => {
                            build_tx_claim_rewards(sdk, source, validator, settings).await?
                        }
                        _ => panic!(),
                    };
                    txs.push((tx, signing_data));
                }

                execute_tx_batch(sdk, txs, task_settings).await?
            }
        }
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
