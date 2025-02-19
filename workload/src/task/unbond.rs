use std::str::FromStr;

use namada_sdk::{
    address::Address,
    args::{self, TxBuilder},
    signing::SigningTxData,
    token,
    tx::{data::GasLimit, Tx},
    Namada,
};
use typed_builder::TypedBuilder;

use crate::state::State;
use crate::{
    check::Check,
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    task::{Amount, Epoch, TaskSettings, ValidatorAddress},
};

use super::utils::get_bond;
use super::{RetryConfig, TaskContext};

#[derive(Clone, TypedBuilder)]
pub struct Unbond {
    source: Alias,
    validator: ValidatorAddress,
    amount: Amount,
    epoch: Epoch,
    settings: TaskSettings,
}

impl TaskContext for Unbond {
    fn name(&self) -> String {
        "unbond".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "unbond/{}/{}/{}",
            self.source.name, self.validator, self.amount
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;

        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", self.source.name)))?;
        let token_amount = token::Amount::from_u64(self.amount);
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        let validator =
            Address::from_str(&self.validator).expect("ValidatorAddress should be converted");

        let mut unbond_tx_builder = sdk
            .namada
            .new_unbond(validator, token_amount)
            .source(source_address.into_owned());
        unbond_tx_builder = unbond_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        unbond_tx_builder = unbond_tx_builder.wrapper_fee_payer(fee_payer);
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        unbond_tx_builder = unbond_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (unbond_tx, signing_data, _epoch) = unbond_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((unbond_tx, vec![signing_data], unbond_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let pre_bond =
            get_bond(sdk, &self.source, &self.validator, self.epoch, retry_config).await?;

        Ok(vec![Check::BondDecrease(
            self.source.clone(),
            self.validator.clone(),
            pre_bond,
            self.amount,
        )])
    }

    fn update_state(&self, state: &mut State, with_fee: bool) {
        if with_fee {
            state.modify_balance_fee(&self.settings.gas_payer, self.settings.gas_limit);
        }
        state.modify_unbonds(&self.source, &self.validator, self.amount);
    }
}
