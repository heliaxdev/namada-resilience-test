use std::str::FromStr;

use namada_sdk::{
    address::Address,
    args::{self, TxBuilder},
    signing::SigningTxData,
    token,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    task::{Address as ValidatorAddress, TaskSettings},
};

use super::utils::execute_tx;

pub async fn build_tx_unbond(
    sdk: &Sdk,
    source: &Alias,
    validator: &ValidatorAddress,
    amount: u64,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;

    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| StepError::Wallet(format!("No source address: {}", source.name)))?;
    let token_amount = token::Amount::from_u64(amount);
    let fee_payer = wallet
        .find_public_key(&settings.gas_payer.name)
        .map_err(|e| StepError::Wallet(e.to_string()))?;
    let validator = Address::from_str(validator).expect("ValidatorAddress should be converted");

    let mut unbond_tx_builder = sdk
        .namada
        .new_unbond(validator, token_amount)
        .source(source_address.into_owned());
    unbond_tx_builder = unbond_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    unbond_tx_builder = unbond_tx_builder.wrapper_fee_payer(fee_payer);
    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    unbond_tx_builder = unbond_tx_builder.signing_keys(signing_keys);
    drop(wallet);

    let (unbond_tx, signing_data, _epoch) = unbond_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((unbond_tx, signing_data, unbond_tx_builder.tx))
}

pub async fn execute_tx_unbond(
    sdk: &Sdk,
    source: &Alias,
    validator: &ValidatorAddress,
    amount: u64,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (unbond_tx, signing_data, tx_args) =
        build_tx_unbond(sdk, source, validator, amount, settings).await?;

    execute_tx(sdk, unbond_tx, vec![signing_data], &tx_args).await
}
