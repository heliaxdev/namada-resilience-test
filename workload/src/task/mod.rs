use std::{
    collections::{BTreeSet, HashMap},
    fmt::Display,
    time::Duration,
};

use enum_dispatch::enum_dispatch;
use namada_sdk::{args, dec::Dec, signing::SigningTxData, tx::Tx};
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{
    build_checks, check::Check, constants::DEFAULT_GAS_LIMIT, entities::Alias, executor::StepError,
    sdk::namada::Sdk,
};

mod batch;
mod become_validator;
mod bond;
mod change_consensus_key;
mod change_metadata;
mod claim_rewards;
mod deactivate_validator;
mod default_proposal;
mod faucet_transfer;
mod init_account;
mod new_wallet_keypair;
mod query_utils;
mod reactivate_validator;
mod redelegate;
mod shielded;
mod shielding;
mod transparent_transfer;
mod tx_utils;
mod unbond;
mod unshielding;
mod update_account;
mod vote;

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
pub type ValidatorAddress = String;
pub type Epoch = u64;
pub type Threshold = u64;
pub type WalletAlias = Alias;
pub type CommissionRate = Dec;
pub type CommissionChange = Dec;
pub type ProposalId = u64;
pub type Vote = String;
type RetryConfig = RetryFutureConfig<ExponentialBackoff, NoOnRetry>;

#[enum_dispatch]
#[derive(Clone)]
pub enum Task {
    NewWalletKeyPair(new_wallet_keypair::NewWalletKeyPair),
    FaucetTransfer(faucet_transfer::FaucetTransfer),
    TransparentTransfer(transparent_transfer::TransparentTransfer),
    Bond(bond::Bond),
    Unbond(unbond::Unbond),
    Redelegate(redelegate::Redelegate),
    ClaimRewards(claim_rewards::ClaimRewards),
    Batch(batch::Batch),
    ShieldedTransfer(shielded::ShieldedTransfer),
    Shielding(shielding::Shielding),
    InitAccount(init_account::InitAccount),
    Unshielding(unshielding::Unshielding),
    BecomeValidator(become_validator::BecomeValidator),
    ChangeMetadata(change_metadata::ChangeMetadata),
    ChangeConsensusKey(change_consensus_key::ChangeConsensusKey),
    DeactivateValidator(deactivate_validator::DeactivateValidator),
    ReactivateValidator(reactivate_validator::ReactivateValidator),
    UpdateAccount(update_account::UpdateAccount),
    DefaultProposal(default_proposal::DefaultProposal),
    Vote(vote::Vote),
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
            Task::ChangeConsensusKey(_, _, _) => "change-consensus-keys".to_string(),
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
            Task::ChangeConsensusKey(_, _, task_settings) => Some(task_settings),
            Task::UpdateAccount(_alias, _, _, task_settings) => Some(task_settings),
            Task::DeactivateValidator(_, task_settings) => Some(task_settings),
            Task::ReactivateValidator(_, task_settings) => Some(task_settings),
            Task::DefaultProposal(_, _, _, _, task_settings) => Some(task_settings),
            Task::Vote(_, _, _, task_settings) => Some(task_settings),
        }
    }

    fn retry_config() -> RetryConfig {
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
            Task::ChangeConsensusKey(alias, _, _) => {
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

#[enum_dispatch(Task)]
trait TaskContext {
    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError>;

    async fn execute(&self, sdk: &Sdk) -> Result<Option<u64>, StepError> {
        let (tx, signing_data, tx_args) = self.build_tx(sdk).await?;
        tx_utils::execute_tx(sdk, tx, signing_data, &tx_args).await
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError>;
}
