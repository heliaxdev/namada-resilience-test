use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, executor::StepError, sdk::namada::Sdk, task::TaskSettings};

use super::utils::execute_tx;

pub async fn build_tx_deactivate_validator(
    sdk: &Sdk,
    target: &Alias,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let target_address = wallet
        .find_address(&target.name)
        .ok_or_else(|| StepError::Wallet(format!("No target address: {}", target.name)))?;
    let fee_payer = wallet
        .find_public_key(&settings.gas_payer.name)
        .map_err(|e| StepError::Wallet(e.to_string()))?;

    let mut deactivate_validator_builder_tx = sdk
        .namada
        .new_deactivate_validator(target_address.into_owned());

    deactivate_validator_builder_tx =
        deactivate_validator_builder_tx.gas_limit(GasLimit::from(settings.gas_limit));
    deactivate_validator_builder_tx = deactivate_validator_builder_tx.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    deactivate_validator_builder_tx = deactivate_validator_builder_tx.signing_keys(signing_keys);

    let (deactivate_validator, signing_data) = deactivate_validator_builder_tx
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((
        deactivate_validator,
        signing_data,
        deactivate_validator_builder_tx.tx,
    ))
}

pub async fn execute_tx_deactivate_validator(
    sdk: &Sdk,
    target: &Alias,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (deactivate_tx, signing_data, tx_args) =
        build_tx_deactivate_validator(sdk, target, settings).await?;

    execute_tx(sdk, deactivate_tx, vec![signing_data], &tx_args).await
}
