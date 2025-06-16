use crate::context::Ctx;
use crate::error::StepError;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task, TaskSettings};
use crate::types::ProposalVote;
use crate::utils::{get_epoch, retry_config};

use super::utils;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Vote;

impl StepContext for Vote {
    fn name(&self) -> String {
        "vote".to_string()
    }

    async fn is_valid(&self, ctx: &Ctx, state: &State) -> Result<bool, StepError> {
        let current_epoch = get_epoch(ctx, retry_config()).await?;
        Ok(state.at_least_bond(1) && state.any_votable_proposal(current_epoch))
    }

    async fn build_task(&self, ctx: &Ctx, state: &State) -> Result<Vec<Task>, StepError> {
        let current_epoch = get_epoch(ctx, retry_config()).await?;
        let Some(proposal_id) = state.random_votable_proposal(current_epoch) else {
            return Ok(vec![]);
        };

        // voter should have bonded at the start epoch
        let start_epoch = state
            .proposals
            .get(&proposal_id)
            .expect("Proposal should exist")
            .0;
        let Some(source_bond) = state.random_bond(start_epoch) else {
            return Ok(vec![]);
        };
        let source_account = state.get_account_by_alias(&source_bond.alias);

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
}
