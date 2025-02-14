use namada_sdk::key::RefTo;
use namada_sdk::{
    args::{self, TxBuilder},
    dec::Dec,
    key::SchemeType,
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};
use rand::rngs::OsRng;

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError, task::TaskSettings};

use super::utils::execute_tx;

#[allow(clippy::too_many_arguments)]
pub async fn build_tx_become_validator(
    sdk: &Sdk,
    source: &Alias,
    consensus_alias: &Alias,
    eth_cold_alias: &Alias,
    eth_hot_alias: &Alias,
    protocol_alias: &Alias,
    commission_rate: Dec,
    commission_max_change: Dec,
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

    let eth_cold_pk = wallet
        .gen_store_secret_key(
            SchemeType::Secp256k1,
            Some(eth_cold_alias.name.clone()),
            true,
            None,
            &mut OsRng,
        )
        .expect("Key generation should not fail.")
        .1
        .ref_to();

    let eth_hot_pk = wallet
        .gen_store_secret_key(
            SchemeType::Secp256k1,
            Some(eth_hot_alias.name.clone()),
            true,
            None,
            &mut OsRng,
        )
        .expect("Key generation should not fail.")
        .1
        .ref_to();

    let protocol_key = wallet
        .gen_store_secret_key(
            SchemeType::Ed25519,
            Some(protocol_alias.name.clone()),
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
    wallet
        .save()
        .map_err(|e| StepError::Wallet(format!("Failed to save the wallet: {e}")))?;

    let mut become_validator_tx_builder = sdk
        .namada
        .new_become_validator(
            source_address.into_owned(),
            commission_rate,
            commission_max_change,
            consensus_pk,
            eth_cold_pk,
            eth_hot_pk,
            protocol_key,
            "test@test.it".to_string(),
        )
        .wallet_alias_force(true);

    become_validator_tx_builder =
        become_validator_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    become_validator_tx_builder = become_validator_tx_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    become_validator_tx_builder = become_validator_tx_builder.signing_keys(signing_keys);
    drop(wallet);

    let (become_validator, signing_data) = become_validator_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((
        become_validator,
        signing_data,
        become_validator_tx_builder.tx,
    ))
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_tx_become_validator(
    sdk: &Sdk,
    source: &Alias,
    consensus_alias: &Alias,
    eth_cold_alias: &Alias,
    eth_hot_alias: &Alias,
    protocol_alias: &Alias,
    commission_rate: Dec,
    commission_max_change: Dec,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (become_validator_tx, signing_data, tx_args) = build_tx_become_validator(
        sdk,
        source,
        consensus_alias,
        eth_cold_alias,
        eth_hot_alias,
        protocol_alias,
        commission_rate,
        commission_max_change,
        settings,
    )
    .await?;

    execute_tx(sdk, become_validator_tx, vec![signing_data], &tx_args).await
}
