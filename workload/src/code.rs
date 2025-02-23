use serde_json::json;

use crate::executor::StepError;
use crate::state::StateError;
use crate::step::StepType;

pub enum Code {
    Success(StepType),
    Fatal(StepType, StepError),
    ExecutionFailure(StepType, StepError),
    BroadcastFailure(StepType, StepError),
    OtherFailure(StepType, StepError),
    BuildFailure(StepType, StepError),
    InvalidStep(StepType),
    NoTask(StepType),
    EmptyBatch(StepType),
    StateFatal(StateError),
    InitFatal(StepError),
}

impl Code {
    pub fn code(&self) -> i32 {
        match self {
            Code::Success(_) | Code::InvalidStep(_) => 0,
            Code::Fatal(_, _) => 1,
            Code::BuildFailure(_, _) => 2,
            Code::ExecutionFailure(_, _) => 3,
            Code::BroadcastFailure(_, _) => 4,
            Code::OtherFailure(_, _) => 5,
            Code::NoTask(_) => 6,
            Code::EmptyBatch(_) => 7,
            Code::StateFatal(_) => 8,
            Code::InitFatal(_) => 9,
        }
    }

    pub fn step_type(&self) -> Option<&StepType> {
        match self {
            Code::Success(st) => Some(st),
            Code::Fatal(st, _) => Some(st),
            Code::ExecutionFailure(st, _) => Some(st),
            Code::BroadcastFailure(st, _) => Some(st),
            Code::OtherFailure(st, _) => Some(st),
            Code::BuildFailure(st, _) => Some(st),
            Code::InvalidStep(st) => Some(st),
            Code::NoTask(st) => Some(st),
            Code::EmptyBatch(st) => Some(st),
            Code::StateFatal(_) => None,
            Code::InitFatal(_) => None,
        }
    }

    pub fn output_logs(&self) {
        match self {
            Code::Success(step_type) => tracing::info!("Done {step_type} successfully!"),
            Code::Fatal(step_type, reason) => {
                tracing::error!("State check error for {step_type} -> {reason}")
            }
            Code::ExecutionFailure(step_type, reason) => {
                tracing::error!("Transaction execution failure for {step_type} -> {reason}")
            }
            Code::BroadcastFailure(step_type, reason) => tracing::info!(
                "Transaction broadcast failure for {step_type} -> {reason}, waiting for next block"
            ),
            Code::OtherFailure(step_type, reason) => {
                tracing::warn!("Failure for {step_type} -> {reason}")
            }
            Code::InvalidStep(step_type) => {
                tracing::warn!("Invalid step for {step_type}, skipping...")
            }
            Code::NoTask(step_type) => tracing::info!("No task for {step_type}, skipping..."),
            Code::BuildFailure(step_type, reason) => {
                tracing::warn!("Build failure for {step_type} -> {reason}")
            }
            Code::EmptyBatch(step_type) => {
                tracing::error!("Building an empty batch for {step_type}")
            }
            Code::StateFatal(reason) => {
                tracing::error!("State error -> {reason}")
            }
            Code::InitFatal(reason) => {
                tracing::error!("Init error -> {reason}")
            }
        }
    }

    pub fn assert(&self) {
        let is_fatal = matches!(self, Code::Fatal(_, _) | Code::StateFatal(_));
        let details = json!({"outcome": self.code()});
        if let Some(step_type) = self.step_type() {
            match step_type {
                StepType::NewWalletKeyPair(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing NewWalletKeyPair",
                        &details
                    );
                }
                StepType::FaucetTransfer(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing FaucetTransfer",
                        &details
                    );
                }
                StepType::TransparentTransfer(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing TransparentTransfer",
                        &details
                    );
                }
                StepType::Bond(_) => {
                    antithesis_sdk::assert_always!(!is_fatal, "Done executing Bond", &details);
                }
                StepType::InitAccount(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing InitAccount",
                        &details
                    );
                }
                StepType::Redelegate(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing Redelegate",
                        &details
                    );
                }
                StepType::Unbond(_) => {
                    antithesis_sdk::assert_always!(!is_fatal, "Done executing Unbond", &details);
                }
                StepType::ClaimRewards(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing ClaimRewards",
                        &details
                    );
                }
                StepType::BatchBond(_) => {
                    antithesis_sdk::assert_always!(!is_fatal, "Done executing BatchBond", &details);
                }
                StepType::BatchRandom(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing BatchRandom",
                        &details
                    );
                }
                StepType::Shielding(_) => {
                    antithesis_sdk::assert_always!(!is_fatal, "Done executing Shielding", &details);
                }
                StepType::Shielded(_) => {
                    antithesis_sdk::assert_always!(!is_fatal, "Done executing Shielded", &details);
                }
                StepType::Unshielding(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing Unshielding",
                        &details
                    );
                }
                StepType::BecomeValidator(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing BecomeValidator",
                        &details
                    );
                }
                StepType::ChangeMetadata(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing ChangeMetadata",
                        &details
                    );
                }
                StepType::ChangeConsensusKey(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing ChangeConsensusKey",
                        &details
                    );
                }
                StepType::UpdateAccount(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing UpdateAccount",
                        &details
                    );
                }
                StepType::DeactivateValidator(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing DeactivateValidator",
                        &details
                    );
                }
                StepType::ReactivateValidator(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing ReactivateValidator",
                        &details
                    );
                }
                StepType::DefaultProposal(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing DefaultProposal",
                        &details
                    );
                }
                StepType::Vote(_) => {
                    antithesis_sdk::assert_always!(
                        !is_fatal,
                        "Done executing VoteProposal",
                        &details
                    );
                }
            }
        }
    }
}
