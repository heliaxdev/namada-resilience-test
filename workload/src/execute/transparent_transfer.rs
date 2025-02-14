use namada_sdk::{
    args::{self, InputAmount, TxBuilder, TxTransparentTransferData},
    signing::SigningTxData,
    token::{self, DenominatedAmount},
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils::execute_tx;

pub async fn build_tx_transparent_transfer(
    sdk: &Sdk,
    source: &Alias,
    target: &Alias,
    amount: u64,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;

    let native_token_alias = Alias::nam();

    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| StepError::Wallet(format!("No source address: {}", source.name)))?;
    let target_address = wallet
        .find_address(&target.name)
        .ok_or_else(|| StepError::Wallet(format!("No target address: {}", target.name)))?;
    let token_address = wallet
        .find_address(&native_token_alias.name)
        .ok_or_else(|| {
            StepError::Wallet(format!(
                "No native token address: {}",
                native_token_alias.name
            ))
        })?;
    let fee_payer = wallet
        .find_public_key(&settings.gas_payer.name)
        .map_err(|e| StepError::Wallet(e.to_string()))?;
    let token_amount = token::Amount::from_u64(amount);

    let tx_transfer_data = TxTransparentTransferData {
        source: source_address.into_owned(),
        target: target_address.into_owned(),
        token: token_address.into_owned(),
        amount: InputAmount::Unvalidated(DenominatedAmount::native(token_amount)),
    };

    let mut transfer_tx_builder = sdk.namada.new_transparent_transfer(vec![tx_transfer_data]);
    transfer_tx_builder = transfer_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(fee_payer);
    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    transfer_tx_builder = transfer_tx_builder.signing_keys(signing_keys);
    drop(wallet);

    let (transfer_tx, signing_data) = transfer_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((transfer_tx, signing_data, transfer_tx_builder.tx))
}

pub async fn execute_tx_transparent_transfer(
    sdk: &Sdk,
    source: &Alias,
    target: &Alias,
    amount: u64,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (transfer_tx, signing_data, tx_args) =
        build_tx_transparent_transfer(sdk, source, target, amount, settings).await?;

    execute_tx(sdk, transfer_tx, vec![signing_data], &tx_args).await
}
