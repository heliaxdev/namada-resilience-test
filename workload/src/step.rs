use clap::ValueEnum;
use std::fmt::Display;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum StepType {
    NewWalletKeyPair,
    FaucetTransfer,
    TransparentTransfer,
    Bond,
    InitAccount,
    Redelegate,
    Unbond,
    ClaimRewards,
    BatchBond,
    BatchRandom,
    Shielding,
    Shielded,
    Unshielding,
    BecomeValidator,
    ChangeMetadata,
    ChangeConsensusKeys,
    UpdateAccount,
    DeactivateValidator,
    ReactivateValidator,
    DefaultProposal,
    VoteProposal,
}

impl Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepType::NewWalletKeyPair => write!(f, "wallet-key-pair"),
            StepType::FaucetTransfer => write!(f, "faucet-transfer"),
            StepType::TransparentTransfer => write!(f, "transparent-transfer"),
            StepType::Bond => write!(f, "bond"),
            StepType::InitAccount => write!(f, "init-account"),
            StepType::Redelegate => write!(f, "redelegate"),
            StepType::Unbond => write!(f, "unbond"),
            StepType::ClaimRewards => write!(f, "claim-rewards"),
            StepType::Shielding => write!(f, "shielding"),
            StepType::BatchRandom => write!(f, "batch-random"),
            StepType::BatchBond => write!(f, "batch-bond"),
            StepType::Shielded => write!(f, "shielded"),
            StepType::Unshielding => write!(f, "unshielding"),
            StepType::BecomeValidator => write!(f, "become-validator"),
            StepType::ChangeMetadata => write!(f, "change-metadata"),
            StepType::ChangeConsensusKeys => write!(f, "change-consensus-keys"),
            StepType::UpdateAccount => write!(f, "update-account"),
            StepType::DeactivateValidator => write!(f, "deactivate-validator"),
            StepType::ReactivateValidator => write!(f, "reactivate-validator"),
            StepType::DefaultProposal => write!(f, "default-proposal"),
            StepType::VoteProposal => write!(f, "vote-proposal"),
        }
    }
}
