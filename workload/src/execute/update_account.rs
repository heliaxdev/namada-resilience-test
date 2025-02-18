use std::collections::BTreeSet;

use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, executor::StepError, sdk::namada::Sdk, task::TaskSettings};

use super::utils::execute_tx;

pub async fn build_tx_update_account(
    sdk: &Sdk,
    target: &Alias,
    sources: &BTreeSet<Alias>,
    threshold: u64,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let target = wallet
        .find_address(&target.name)
        .ok_or_else(|| StepError::Wallet(format!("No target address: {}", target.name)))?;

    let mut public_keys = vec![];
    for source in sources {
        let source_pk = wallet
            .find_public_key(&source.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        public_keys.push(source_pk);
    }

    let fee_payer = wallet
        .find_public_key(&settings.gas_payer.name)
        .map_err(|e| StepError::Wallet(e.to_string()))?;

    let mut update_account_builder =
        sdk.namada
            .new_update_account(target.into_owned(), public_keys, threshold as u8);

    update_account_builder = update_account_builder.gas_limit(GasLimit::from(settings.gas_limit));
    update_account_builder = update_account_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    update_account_builder = update_account_builder.signing_keys(signing_keys);
    drop(wallet);

    let (update_account, signing_data) = update_account_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((update_account, signing_data, update_account_builder.tx))
}

pub async fn execute_tx_update_account(
    sdk: &Sdk,
    target: &Alias,
    sources: &BTreeSet<Alias>,
    threshold: u64,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (update_account_tx, signing_data, tx_args) =
        build_tx_update_account(sdk, target, sources, threshold, settings).await?;

    execute_tx(sdk, update_account_tx, vec![signing_data], &tx_args).await
}
