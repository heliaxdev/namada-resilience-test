use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{
    check::Check,
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    task::{ProposalId, TaskSettings, Vote as VoteOption},
};

use super::{RetryConfig, TaskContext};

#[derive(Clone, Debug)]
pub(super) struct Vote {
    source: Alias,
    proposal_id: ProposalId,
    vote: VoteOption,
    settings: TaskSettings,
}

impl TaskContext for Vote {
    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;
        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", self.source.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;

        let mut vote_tx_builder = sdk.namada.new_proposal_vote(
            self.proposal_id,
            self.vote.clone(),
            source_address.into_owned(),
        );
        vote_tx_builder = vote_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        vote_tx_builder = vote_tx_builder.wrapper_fee_payer(fee_payer);
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        vote_tx_builder = vote_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (vote_tx, signing_data) = vote_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((vote_tx, vec![signing_data], vote_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        Ok(vec![])
    }
}
