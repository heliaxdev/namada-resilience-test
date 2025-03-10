use namada_sdk::args::{self, InputAmount, TxBuilder, TxShieldedTransferData};
use namada_sdk::masp_primitives;
use namada_sdk::masp_primitives::transaction::components::sapling::builder::RngBuildParams;
use namada_sdk::masp_primitives::zip32::PseudoExtendedKey;
use namada_sdk::signing::SigningTxData;
use namada_sdk::token;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use rand::rngs::OsRng;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Amount};
use crate::utils::{get_shielded_balance, shielded_sync_with_retry, RetryConfig};

#[derive(Clone, Debug, TypedBuilder)]
pub struct ShieldedTransfer {
    source: Alias,
    target: Alias,
    amount: Amount,
    settings: TaskSettings,
}

impl TaskContext for ShieldedTransfer {
    fn name(&self) -> String {
        "shielded-transfer".to_string()
    }

    fn summary(&self) -> String {
        format!(
            "shielded-transfer/{}/{}/{}",
            self.source.name, self.target.name, self.amount
        )
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let mut bparams = RngBuildParams::new(OsRng);
        let mut wallet = sdk.namada.wallet.write().await;

        let native_token_alias = Alias::nam();

        let source_spending_key = wallet
            .find_spending_key(&self.source.name, None)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        let tmp = masp_primitives::zip32::ExtendedSpendingKey::from(source_spending_key);
        let pseudo_spending_key_from_spending_key = PseudoExtendedKey::from(tmp);
        let target_payment_address =
            *wallet.find_payment_addr(&self.target.name).ok_or_else(|| {
                StepError::Wallet(format!("No payment address: {}", self.target.name))
            })?;
        let token = wallet
            .find_address(&native_token_alias.name)
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "No native token address: {}",
                    native_token_alias.name
                ))
            })?;
        let token_amount = token::Amount::from_u64(self.amount);
        let amount = InputAmount::Unvalidated(token::DenominatedAmount::native(token_amount));
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        let tx_transfer_data = TxShieldedTransferData {
            source: pseudo_spending_key_from_spending_key,
            target: target_payment_address,
            token: token.into_owned(),
            amount,
        };

        // FIXME review the gaspayer
        let mut transfer_tx_builder =
            sdk.namada
                .new_shielded_transfer(vec![tx_transfer_data], None, false);
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
            .map_err(|e| StepError::BuildTx(e.to_string()))?;

        Ok((transfer_tx, vec![signing_data], transfer_tx_builder.tx))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        shielded_sync_with_retry(sdk, &self.source, None, false).await?;

        let pre_balance = get_shielded_balance(sdk, &self.source, retry_config)
            .await?
            .unwrap_or_default();
        let source_check = Check::BalanceShieldedSource(
            check::balance_shielded_source::BalanceShieldedSource::builder()
                .target(self.source.clone())
                .pre_balance(pre_balance)
                .amount(self.amount)
                .build(),
        );

        let pre_balance = get_shielded_balance(sdk, &self.target, retry_config)
            .await?
            .unwrap_or_default();
        let target_check = Check::BalanceShieldedTarget(
            check::balance_shielded_target::BalanceShieldedTarget::builder()
                .target(self.target.clone())
                .pre_balance(pre_balance)
                .amount(self.amount)
                .build(),
        );

        Ok(vec![source_check, target_check])
    }

    fn update_state(&self, state: &mut State, with_fee: bool) {
        if with_fee {
            state.modify_balance_fee(&self.settings.gas_payer, self.settings.gas_limit);
        }
        state.modify_shielded_transfer(&self.source, &self.target, self.amount);
    }
}
