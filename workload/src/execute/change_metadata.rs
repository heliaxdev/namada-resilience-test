use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{entities::Alias, executor::StepError, sdk::namada::Sdk, task::TaskSettings};

use super::utils::execute_tx;

#[allow(clippy::too_many_arguments)]
pub async fn build_tx_change_metadata(
    sdk: &Sdk,
    source: &Alias,
    website: &str,
    email: &str,
    discord: &str,
    description: &str,
    avatar: &str,
    settings: &TaskSettings,
) -> Result<(Tx, SigningTxData, args::Tx), StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| StepError::Wallet(format!("No source address: {}", source.name)))?;
    let fee_payer = wallet
        .find_public_key(&settings.gas_payer.name)
        .map_err(|e| StepError::Wallet(e.to_string()))?;

    let mut change_metadata_tx_builder = sdk
        .namada
        .new_change_metadata(source_address.into_owned())
        .avatar(avatar.to_string())
        .description(description.to_string())
        .discord_handle(discord.to_string())
        .email(email.to_string())
        .website(website.to_string());

    change_metadata_tx_builder =
        change_metadata_tx_builder.gas_limit(GasLimit::from(settings.gas_limit));
    change_metadata_tx_builder = change_metadata_tx_builder.wrapper_fee_payer(fee_payer);

    let mut signing_keys = vec![];
    for signer in &settings.signers {
        let public_key = wallet
            .find_public_key(&signer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        signing_keys.push(public_key)
    }
    change_metadata_tx_builder = change_metadata_tx_builder.signing_keys(signing_keys);
    drop(wallet);

    let (change_metadata, signing_data) = change_metadata_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    Ok((change_metadata, signing_data, change_metadata_tx_builder.tx))
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_tx_change_metadata(
    sdk: &Sdk,
    source: &Alias,
    website: &str,
    email: &str,
    discord: &str,
    description: &str,
    avatar: &str,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (change_metadata_tx, signing_data, tx_args) = build_tx_change_metadata(
        sdk,
        source,
        website,
        email,
        discord,
        description,
        avatar,
        settings,
    )
    .await?;

    execute_tx(sdk, change_metadata_tx, vec![signing_data], &tx_args).await
}
