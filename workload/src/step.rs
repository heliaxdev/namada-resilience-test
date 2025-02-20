use std::fmt::Display;

use enum_dispatch::enum_dispatch;
use rand::{distributions::Standard, prelude::Distribution, Rng};

use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::Task;

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
mod reactivate_validator;
mod redelegate;
mod shielded_transfer;
mod shielding;
mod transparent_transfer;
mod unbond;
mod unshielding;
mod update_account;
mod utils;
mod vote;

#[enum_dispatch]
#[derive(Debug)]
pub enum StepType {
    NewWalletKeyPair(new_wallet_keypair::NewWalletKeyPair),
    FaucetTransfer(faucet_transfer::FaucetTransfer),
    TransparentTransfer(transparent_transfer::TransparentTransfer),
    Bond(bond::Bond),
    InitAccount(init_account::InitAccount),
    Redelegate(redelegate::Redelegate),
    Unbond(unbond::Unbond),
    ClaimRewards(claim_rewards::ClaimRewards),
    BatchBond(batch::BatchBond),
    BatchRandom(batch::BatchRandom),
    Shielding(shielding::Shielding),
    Shielded(shielded_transfer::ShieldedTransfer),
    Unshielding(unshielding::Unshielding),
    BecomeValidator(become_validator::BecomeValidator),
    ChangeMetadata(change_metadata::ChangeMetadata),
    ChangeConsensusKey(change_consensus_key::ChangeConsensusKey),
    UpdateAccount(update_account::UpdateAccount),
    DeactivateValidator(deactivate_validator::DeactivateValidator),
    ReactivateValidator(reactivate_validator::ReactivateValidator),
    DefaultProposal(default_proposal::DefaultProposal),
    Vote(vote::Vote),
}

impl Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Distribution<StepType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> StepType {
        match rng.gen_range(0..6) {
            0 => StepType::TransparentTransfer(Default::default()),
            1 => StepType::Redelegate(Default::default()),
            2 => StepType::Unbond(Default::default()),
            3 => StepType::Shielding(Default::default()),
            4 => StepType::ClaimRewards(Default::default()),
            5 => StepType::Bond(Default::default()),
            6 => StepType::Shielded(Default::default()),
            _ => StepType::Unshielding(Default::default()),
        }
    }
}

#[enum_dispatch(StepType)]
pub trait StepContext {
    fn name(&self) -> String;

    async fn is_valid(&self, sdk: &Sdk, state: &State) -> Result<bool, StepError>;

    async fn build_task(&self, sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError>;
}
