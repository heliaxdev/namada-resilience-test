use std::str::FromStr;

use namada_sdk::{
    address::Address,
    args::{self, TxBuilder},
    signing::SigningTxData,
    token,
    tx::{data::GasLimit, Tx},
    Namada,
};

use crate::{
    check::Check,
    entities::Alias,
    executor::StepError,
    sdk::namada::Sdk,
    task::TaskSettings,
    task::{Amount, Epoch, ValidatorAddress},
};

use super::query_utils::get_bond;
use super::{RetryConfig, TaskContext};

#[derive(Clone, Debug)]
pub(super) struct Bond {
    source: Alias,
    validator: ValidatorAddress,
    amount: Amount,
    epoch: Epoch,
    settings: TaskSettings,
}

impl TaskContext for Bond {
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

        let mut bond_tx_builder = sdk
            .namada
            .new_bond(validator, token_amount)
            .source(source_address.into_owned());
        bond_tx_builder = bond_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        bond_tx_builder = bond_tx_builder.wrapper_fee_payer(fee_payer);
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        bond_tx_builder = bond_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (bond_tx, signing_data) = bond_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((bond_tx, vec![signing_data], bond_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let pre_bond =
            get_bond(sdk, &self.source, &self.validator, self.epoch, retry_config).await?;

        Ok(vec![Check::BondIncrease(
            self.source.clone(),
            self.validator.clone(),
            pre_bond,
            self.amount,
        )])
    }
}
