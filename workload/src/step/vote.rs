use crate::code::{Code, CodeType};
use crate::error::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::ProposalVote;
use crate::utils::{get_epoch, retry_config};
use crate::{assert_always_step, assert_sometimes_step, assert_unreachable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct Vote;

impl StepContext for Vote {
    fn name(&self) -> String {
        "vote".to_string()
    }

    async fn is_valid(&self, sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        let current_epoch = get_epoch(sdk, retry_config()).await?;
        Ok(state.any_bond() && state.any_votable_proposal(current_epoch))
    }

    async fn build_task(&self, sdk: &Sdk, state: &State) -> Result<Vec<Task>, StepError> {
        let source_bond = state.random_bond();
        let source_account = state.get_account_by_alias(&source_bond.alias);

        let current_epoch = get_epoch(sdk, retry_config()).await?;

        let proposal_id = state.random_votable_proposal(current_epoch);

        let vote = if utils::coin_flip(0.5) {
            ProposalVote::Yay
        } else if utils::coin_flip(0.5) {
            ProposalVote::Nay
        } else {
            ProposalVote::Abstain
        };

        let gas_payer = utils::get_gas_payer(source_account.public_keys.iter(), state);
        let mut task_settings = TaskSettings::new(source_account.public_keys, gas_payer);
        task_settings.gas_limit *= 5;

        Ok(vec![Task::Vote(
            task::vote::Vote::builder()
                .source(source_account.alias)
                .proposal_id(proposal_id)
                .vote(vote)
                .settings(task_settings)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        match code.code_type() {
            CodeType::Success => assert_always_step!("Done Vote", code),
            CodeType::Fatal => assert_unreachable_step!("Fatal Vote", code),
            CodeType::Skip => assert_sometimes_step!("Skipped Vote", code),
            CodeType::Failed => assert_unreachable_step!("Failed Vote", code),
        }
    }
}
