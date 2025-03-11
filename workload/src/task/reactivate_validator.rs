use namada_sdk::args::{self, TxBuilder};
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, ValidatorStatus};
use crate::utils::RetryConfig;

#[derive(Clone, Debug, TypedBuilder)]
pub struct ReactivateValidator {
    target: Alias,
    settings: TaskSettings,
}

impl TaskContext for ReactivateValidator {
    fn name(&self) -> String {
        "reactivate-validator".to_string()
    }

    fn summary(&self) -> String {
        format!("reactivate-validator/{}", self.target.name)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| StepError::Wallet(format!("No target address: {}", self.target.name)))?;

        let mut reactivate_validator_builder_tx = sdk
            .namada
            .new_reactivate_validator(target_address.into_owned());

        reactivate_validator_builder_tx =
            reactivate_validator_builder_tx.gas_limit(GasLimit::from(self.settings.gas_limit));

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
            .map_err(|e| StepError::BuildTx(e.to_string()))?;

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
            check::validator_status::ValidatorStatus::builder()
                .target(self.target.clone())
                .status(ValidatorStatus::Reactivating)
                .build(),
        )])
    }

    fn update_state(&self, state: &mut State) {
        state.remove_deactivate_validator(&self.target);
    }
}
