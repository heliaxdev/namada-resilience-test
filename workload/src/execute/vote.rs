use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils::execute_tx;

pub async fn build_tx_vote(
    sdk: &Sdk,
    source: &Alias,
    proposal_id: u64,
    vote: &str,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| StepError::Wallet(format!("No source address: {}", source.name)))?;
    let fee_payer = wallet
        .find_public_key(&settings.gas_payer.name)
        .map_err(|e| StepError::Wallet(e.to_string()))?;

    let mut vote_tx_builder =
        sdk.namada
            .new_proposal_vote(proposal_id, vote.to_string(), source_address.into_owned());
    vote_tx_builder = vote_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    vote_tx_builder = vote_tx_builder.wrapper_fee_payer(fee_payer);
    let mut signing_keys = vec![];
    for signer in &settings.signers {
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

    Ok((vote_tx, signing_data, vote_tx_builder.tx))
}

pub async fn execute_tx_vote(
    sdk: &Sdk,
    source: &Alias,
    proposal_id: u64,
    vote: &str,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (vote_tx, signing_data, tx_args) =
        build_tx_vote(sdk, source, proposal_id, vote, settings).await?;

    execute_tx(sdk, vote_tx, vec![signing_data], &tx_args).await
}
