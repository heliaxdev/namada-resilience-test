use std::collections::BTreeSet;

use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{
    check::Check,
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    task::{TaskSettings, Threshold},
};

use super::{RetryConfig, TaskContext};

#[derive(Clone, Debug)]
pub(super) struct UpdateAccount {
    target: Alias,
    sources: BTreeSet<Alias>,
    threshold: Threshold,
    settings: TaskSettings,
}

impl TaskContext for UpdateAccount {
    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;
        let target = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| StepError::Wallet(format!("No target address: {}", self.target.name)))?;

        let mut public_keys = vec![];
        for source in &self.sources {
            let source_pk = wallet
                .find_public_key(&source.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            public_keys.push(source_pk);
        }

        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;

        let mut update_account_builder =
            sdk.namada
                .new_update_account(target.into_owned(), public_keys, self.threshold as u8);

        update_account_builder =
            update_account_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        update_account_builder = update_account_builder.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
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

        Ok((
            update_account,
            vec![signing_data],
            update_account_builder.tx,
        ))
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        Ok(vec![Check::AccountExist(
            self.target.clone(),
            self.threshold,
            self.sources.clone(),
        )])
    }
}
