use serde_json::json;
use typed_builder::TypedBuilder;

use crate::check::{CheckContext, CheckInfo};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::types::{Alias, ProposalId, ProposalVote};
use crate::utils::{get_vote_results, RetryConfig};

#[derive(TypedBuilder)]
pub struct VoteResult {
    source: Alias,
    proposal_id: ProposalId,
    vote: ProposalVote,
}

impl CheckContext for VoteResult {
    fn summary(&self) -> String {
        format!("vote-result/{}", self.source.name)
    }

    async fn do_check(
        &self,
        sdk: &Sdk,
        check_info: CheckInfo,
        retry_config: RetryConfig,
    ) -> Result<(), StepError> {
        let votes = get_vote_results(sdk, &self.source, self.proposal_id, retry_config).await?;

        let is_valid_vote = votes.iter().all(|v| *v == self.vote);

        let details = json!({
            "target_alias": self.source,
            "proposal_id": self.proposal_id,
            "vote": self.vote,
            "execution_height": check_info.execution_height,
            "check_height": check_info.check_height
        });

        antithesis_sdk::assert_always_or_unreachable!(
            is_valid_vote,
            "Vote was accepted as expected",
            &details
        );

        if is_valid_vote {
            Ok(())
        } else {
            tracing::error!("{}", details);
            Err(StepError::StateCheck(format!("VoteResult check error: Vote result {votes:?} doesn't correspond to the expected vote {}", self.vote)))
        }
    }
}
