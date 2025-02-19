use namada_sdk::{
    args::{self, TxBuilder},
    signing::SigningTxData,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{
    check::{Check, ValidatorStatus},
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    task::TaskSettings,
};

use super::{RetryConfig, TaskContext};

#[derive(Clone, Debug)]
pub(super) struct ReactivateValidator {
    target: Alias,
    settings: TaskSettings,
}

impl TaskContext for ReactivateValidator {
    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| StepError::Wallet(format!("No target address: {}", self.target.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;

        let mut reactivate_validator_builder_tx = sdk
            .namada
            .new_reactivate_validator(target_address.into_owned());

        reactivate_validator_builder_tx =
            reactivate_validator_builder_tx.gas_limit(GasLimit::from(self.settings.gas_limit));
        reactivate_validator_builder_tx =
            reactivate_validator_builder_tx.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        reactivate_validator_builder_tx =
            reactivate_validator_builder_tx.signing_keys(signing_keys);

        let (reactivate_validator, signing_data) = reactivate_validator_builder_tx
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((
            reactivate_validator,
            vec![signing_data],
            reactivate_validator_builder_tx.tx,
        ))
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        Ok(vec![Check::ValidatorStatus(
            self.target.clone(),
            ValidatorStatus::Reactivating,
        )])
    }
}
