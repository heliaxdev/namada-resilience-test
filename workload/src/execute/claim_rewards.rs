use std::str::FromStr;

use namada_sdk::{
    address::Address,
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{
    entities::Alias,
    sdk::namada::Sdk,
    steps::StepError,
    task::{Address as ValidatorAddress, TaskSettings},
};

use super::utils::execute_tx;

pub async fn build_tx_claim_rewards(
    sdk: &Sdk,
    source: &Alias,
    from_validator: &ValidatorAddress,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;

    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| StepError::Wallet(format!("No source address: {}", source.name)))?;
    let fee_payer = wallet
        .find_public_key(&settings.gas_payer.name)
        .map_err(|e| StepError::Wallet(e.to_string()))?;
    let from_validator =
        Address::from_str(from_validator).expect("ValidatorAddress should be converted");

    let mut claim_rewards_tx_builder = sdk.namada.new_claim_rewards(from_validator);
    claim_rewards_tx_builder.source = Some(source_address.into_owned());
    claim_rewards_tx_builder =
        claim_rewards_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    claim_rewards_tx_builder = claim_rewards_tx_builder.wrapper_fee_payer(fee_payer);
    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    claim_rewards_tx_builder = claim_rewards_tx_builder.signing_keys(signing_keys);
    drop(wallet);

    let (claim_tx, signing_data) = claim_rewards_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((claim_tx, signing_data, claim_rewards_tx_builder.tx))
}

pub async fn execute_tx_claim_rewards(
    sdk: &Sdk,
    source: &Alias,
    from_validator: &ValidatorAddress,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (claim_tx, signing_data, tx_args) =
        build_tx_claim_rewards(sdk, source, from_validator, settings).await?;

    execute_tx(sdk, claim_tx, vec![signing_data], &tx_args).await
}
