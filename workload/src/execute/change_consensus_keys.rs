use namada_sdk::{
    args::{self, TxBuilder},
    key::{RefTo, SchemeType},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};
use rand::rngs::OsRng;

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils::execute_tx;

pub async fn build_tx_change_consensus_key(
    sdk: &Sdk,
    source: &Alias,
    consensus_alias: &Alias,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let mut wallet = sdk.namada.wallet.write().await;

    let consensus_pk = wallet
        .gen_store_secret_key(
            SchemeType::Ed25519,
            Some(consensus_alias.name.clone()),
            true,
            None,
            &mut OsRng,
        )
        .expect("Key generation should not fail.")
        .1
        .ref_to();

    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| StepError::Wallet(format!("No source address: {}", source.name)))?;
    let fee_payer = wallet
        .find_public_key(&settings.gas_payer.name)
        .map_err(|e| StepError::Wallet(e.to_string()))?;

    let mut change_consensus_key_builder = sdk
        .namada
        .new_change_consensus_key(source_address.into_owned(), consensus_pk);

    change_consensus_key_builder =
        change_consensus_key_builder.gas_limit(GasLimit::from(settings.gas_limit));
    change_consensus_key_builder = change_consensus_key_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    change_consensus_key_builder = change_consensus_key_builder.signing_keys(signing_keys);
    drop(wallet);

    let (change_consensus_key, signing_data) = change_consensus_key_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((
        change_consensus_key,
        signing_data,
        change_consensus_key_builder.tx,
    ))
}

pub async fn execute_tx_change_consensus_key(
    sdk: &Sdk,
    source: &Alias,
    consensus_alias: &Alias,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (change_consensus_key_tx, signing_data, tx_args) =
        build_tx_change_consensus_key(sdk, source, consensus_alias, settings).await?;

    execute_tx(sdk, change_consensus_key_tx, vec![signing_data], &tx_args).await
}
