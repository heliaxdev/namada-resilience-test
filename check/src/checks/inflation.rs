use namada_sdk::governance::utils::{ProposalStatus, TallyResult};
use namada_sdk::rpc;
use namada_sdk::token::Amount;

use crate::sdk::namada::Sdk;
use crate::state::State;

use super::DoCheck;

const PROPOSAL_DEPOSIT: u64 = 50 * namada_sdk::token::NATIVE_SCALE;

#[derive(Clone, Debug, Default)]
pub struct InflationCheck;

impl DoCheck for InflationCheck {
    async fn check(&self, sdk: &Sdk, state: &mut State) -> Result<(), String> {
        let native_token = rpc::query_native_token(&sdk.namada.client)
            .await
            .map_err(|e| e.to_string())?;
        let current_total_supply = rpc::get_token_total_supply(&sdk.namada.client, &native_token)
            .await
            .map_err(|e| format!("Failed to query total supply: {e}"))?;

        let rejected = count_rejected_proposals(sdk, state).await?;
        let burned_amount = Amount::from_u64(rejected * PROPOSAL_DEPOSIT);
        let last_total_supply = state
            .last_total_supply
            .checked_sub(burned_amount)
            .unwrap_or_default();

        if last_total_supply <= current_total_supply {
            state.last_total_supply = current_total_supply;
            tracing::info!("Total supply ok");
            Ok(())
        } else {
            Err(format!(
                "Total supply decreases: before: {} -> after {}",
                last_total_supply, current_total_supply
            ))
        }
    }

    fn timing(&self) -> u32 {
        20
    }

    fn name(&self) -> String {
        "InflationCheck".to_string()
    }
}

async fn count_rejected_proposals(sdk: &Sdk, state: &mut State) -> Result<u64, String> {
    let client = &sdk.namada.client;

    let mut rejected = 0;
    let mut proposal_id = state.last_end_proposal_id.map_or(0, |last_id| last_id + 1);
    loop {
        let proposal = rpc::query_proposal_by_id(client, proposal_id)
            .await
            .map_err(|e| e.to_string())?;
        let Some(proposal) = proposal else {
            return Ok(0);
        };

        let epoch = rpc::query_epoch(client).await.map_err(|e| e.to_string())?;
        if matches!(proposal.get_status(epoch), ProposalStatus::Ended) {
            state.last_end_proposal_id = Some(proposal_id);

            let result = rpc::query_proposal_result(client, proposal_id)
                .await
                .map_err(|e| e.to_string())?
                .expect("Proposal should exist");
            if matches!(result.result, TallyResult::Rejected) {
                rejected += 1;
            }
        } else {
            break;
        }

        proposal_id += 1;
    }

    Ok(rejected)
}
