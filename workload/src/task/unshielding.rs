use namada_sdk::{
    args::{self, InputAmount, TxBuilder, TxUnshieldingTransferData},
    masp_primitives::{
        self, transaction::components::sapling::builder::RngBuildParams, zip32::PseudoExtendedKey,
    },
    signing::SigningTxData,
    token,
    tx::{data::GasLimit, Tx},
    Namada,
};
use rand::rngs::OsRng;
use typed_builder::TypedBuilder;

use crate::check::Check;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Amount};
use crate::utils::{get_balance, get_shielded_balance, RetryConfig};

#[derive(Clone, TypedBuilder)]
pub struct Unshielding {
    source: Alias,
    target: Alias,
    amount: Amount,
    settings: TaskSettings,
}

impl TaskContext for Unshielding {
    fn name(&self) -> String {
        "unshielding".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "unshielding/{}/{}/{}",
            self.source.name, self.target.name, self.amount
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let mut bparams = RngBuildParams::new(OsRng);

        let mut wallet = sdk.namada.wallet.write().await;

        let source_spending_key = wallet
            .find_spending_key(&self.source.name, None)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        let tmp = masp_primitives::zip32::ExtendedSpendingKey::from(source_spending_key);
        let pseudo_spending_key_from_spending_key = PseudoExtendedKey::from(tmp);
        let target_address = wallet
            .find_address(&self.target.name)
            .ok_or_else(|| StepError::Wallet(format!("No target address: {}", self.target.name)))?;

        let native_token_alias = Alias::nam();
        let token = wallet
            .find_address(&native_token_alias.name)
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "No native token address: {}",
                    native_token_alias.name
                ))
            })?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        let token_amount = token::Amount::from_u64(self.amount);
        let amount = InputAmount::Unvalidated(token::DenominatedAmount::native(token_amount));

        let tx_transfer_data = TxUnshieldingTransferData {
            target: target_address.into_owned(),
            token: token.into_owned(),
            amount,
        };

        let mut transfer_tx_builder = sdk.namada.new_unshielding_transfer(
            pseudo_spending_key_from_spending_key,
            vec![tx_transfer_data],
            None,
            false,
        );

        transfer_tx_builder =
            transfer_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        transfer_tx_builder = transfer_tx_builder.wrapper_fee_payer(fee_payer);
        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        transfer_tx_builder = transfer_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (transfer_tx, signing_data) = transfer_tx_builder
            .build(&sdk.namada, &mut bparams)
            .await
            .map_err(|e| StepError::Build(e.to_string()))?;

        Ok((transfer_tx, vec![signing_data], transfer_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let pre_balance = get_shielded_balance(sdk, &self.source, None, false)
            .await?
            .unwrap_or_default();
        let source_check =
            Check::BalanceShieldedSource(self.source.clone(), pre_balance, self.amount);

        let (_, pre_balance) = get_balance(sdk, &self.target, retry_config).await?;
        let target_check = Check::BalanceTarget(self.target.clone(), pre_balance, self.amount);

        Ok(vec![source_check, target_check])
    }

    fn update_state(&self, state: &mut State, with_fee: bool) {
        if with_fee {
            state.modify_balance_fee(&self.settings.gas_payer, self.settings.gas_limit);
        }
        state.modify_unshielding(&self.source, &self.target, self.amount)
    }
}
