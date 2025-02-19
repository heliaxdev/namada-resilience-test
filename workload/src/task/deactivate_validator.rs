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
pub(super) struct DeactivateValidator {
    target: Alias,
    settings: TaskSettings,
}

impl TaskContext for DeactivateValidator {
    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| StepError::Wallet(format!("No target address: {}", self.target.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;

        let mut deactivate_validator_builder_tx = sdk
            .namada
            .new_deactivate_validator(target_address.into_owned());

        deactivate_validator_builder_tx =
            deactivate_validator_builder_tx.gas_limit(GasLimit::from(self.settings.gas_limit));
        deactivate_validator_builder_tx =
            deactivate_validator_builder_tx.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        deactivate_validator_builder_tx =
            deactivate_validator_builder_tx.signing_keys(signing_keys);

        let (deactivate_validator, signing_data) = deactivate_validator_builder_tx
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((
            deactivate_validator,
            vec![signing_data],
            deactivate_validator_builder_tx.tx,
        ))
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        Ok(vec![Check::ValidatorStatus(
            self.target.clone(),
            ValidatorStatus::Inactive,
        )])
    }
}
