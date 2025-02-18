use std::collections::BTreeSet;

use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, executor::StepError, sdk::namada::Sdk, task::TaskSettings};

use super::utils::execute_tx;

pub async fn build_tx_init_account(
    sdk: &Sdk,
    target: &Alias,
    sources: &BTreeSet<Alias>,
    threshold: u64,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;

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

    let mut init_account_builder = sdk
        .namada
        .new_init_account(public_keys, Some(threshold as u8))
        .initialized_account_alias(target.name.clone())
        .wallet_alias_force(true);

    init_account_builder = init_account_builder.gas_limit(GasLimit::from(settings.gas_limit));
    init_account_builder = init_account_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    init_account_builder = init_account_builder.signing_keys(signing_keys);
    drop(wallet);

    let (init_account_tx, signing_data) = init_account_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((init_account_tx, signing_data, init_account_builder.tx))
}

pub async fn execute_tx_init_account(
    sdk: &Sdk,
    target: &Alias,
    sources: &BTreeSet<Alias>,
    threshold: u64,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (init_account_tx, signing_data, tx_args) =
        build_tx_init_account(sdk, target, sources, threshold, settings).await?;

    execute_tx(sdk, init_account_tx, vec![signing_data], &tx_args).await
}
