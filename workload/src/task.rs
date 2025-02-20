use std::{collections::BTreeSet, fmt::Display};

use enum_dispatch::enum_dispatch;
use namada_sdk::{args, signing::SigningTxData, tx::Tx};

use crate::check::Check;
use crate::constants::DEFAULT_GAS_LIMIT;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::types::Alias;
use crate::utils;
use crate::utils::RetryConfig;

pub mod batch;
pub mod become_validator;
pub mod bond;
pub mod change_consensus_key;
pub mod change_metadata;
pub mod claim_rewards;
pub mod deactivate_validator;
pub mod default_proposal;
pub mod faucet_transfer;
pub mod init_account;
pub mod new_wallet_keypair;
pub mod reactivate_validator;
pub mod redelegate;
pub mod shielded;
pub mod shielding;
pub mod transparent_transfer;
pub mod unbond;
pub mod unshielding;
pub mod update_account;
pub mod vote;

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

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

#[enum_dispatch(Task)]
pub trait TaskContext {
    fn name(&self) -> String;

    fn summary(&self) -> String;

    fn task_settings(&self) -> Option<&TaskSettings>;

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError>;

    async fn execute(&self, sdk: &Sdk) -> Result<Option<u64>, StepError> {
        let (tx, signing_data, tx_args) = self.build_tx(sdk).await?;
        utils::execute_tx(sdk, tx, signing_data, &tx_args).await
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError>;

    fn update_state(&self, state: &mut State, with_fee: bool);

    fn update_stats(&self, state: &mut State) {
        state
            .stats
            .entry(self.name())
            .and_modify(|counter| *counter += 1)
            .or_insert(1);
    }

    fn update_failed_execution(&self, state: &mut State) {
        if let Some(settings) = self.task_settings() {
            state.modify_balance_fee(&settings.gas_payer, settings.gas_limit);
        }
    }
}
