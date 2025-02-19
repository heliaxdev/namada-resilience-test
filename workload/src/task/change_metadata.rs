use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{
    check::Check, entities::Alias, executor::StepError, sdk::namada::Sdk, task::TaskSettings,
};

use super::{RetryConfig, TaskContext};

#[derive(Clone, Debug)]
pub(super) struct ChangeMetadata {
    source: Alias,
    website: String,
    email: String,
    discord: String,
    description: String,
    avatar: String,
    settings: TaskSettings,
}

impl TaskContext for ChangeMetadata {
    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;
        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", self.source.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;

        let mut change_metadata_tx_builder = sdk
            .namada
            .new_change_metadata(source_address.into_owned())
            .avatar(self.avatar.clone())
            .description(self.description.clone())
            .discord_handle(self.discord.clone())
            .email(self.email.clone())
            .website(self.website.clone());

        change_metadata_tx_builder =
            change_metadata_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        change_metadata_tx_builder = change_metadata_tx_builder.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
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

        Ok((
            change_metadata,
            vec![signing_data],
            change_metadata_tx_builder.tx,
        ))
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        Ok(vec![])
    }
}
