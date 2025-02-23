use namada_sdk::key::SchemeType;
use namada_sdk::masp::find_valid_diversifier;
use namada_sdk::masp_primitives::zip32;
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::Tx;
use namada_sdk::{args, rpc, PaymentAddress};
use rand::rngs::OsRng;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::Alias;
use crate::utils::{build_reveal_pk, RetryConfig};

#[derive(Clone, Debug, TypedBuilder)]
pub struct NewWalletKeyPair {
    source: Alias,
}

impl TaskContext for NewWalletKeyPair {
    fn name(&self) -> String {
        "new-wallet-key-pair".to_string()
    }

    fn summary(&self) -> String {
        format!("new-wallet-key-pair/{}", self.source.name)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        None
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let block = rpc::query_block(&sdk.namada.client)
            .await
            .map_err(StepError::Rpc)?
            .ok_or_else(|| StepError::StateCheck("No block found".to_string()))?;

        let mut wallet = sdk.namada.wallet.write().await;

        let (_alias, sk) = wallet
            .gen_store_secret_key(
                SchemeType::Ed25519,
                Some(self.source.name.clone()),
                true,
                None,
                &mut OsRng,
            )
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "Failed to generate keypair for {}",
                    self.source.name
                ))
            })?;

        let spending_key_alias = format!("{}-spending-key", self.source.name);
        let (_alias, spending_key) = wallet
            .gen_store_spending_key(
                spending_key_alias.clone(),
                Some(block.height),
                None,
                true,
                &mut OsRng,
            )
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "Failed to generate spending key for {}",
                    spending_key_alias
                ))
            })?;

        let viewing_key = zip32::ExtendedFullViewingKey::from(&spending_key.into())
            .fvk
            .vk;
        let (div, _g_d) = find_valid_diversifier(&mut OsRng);
        let masp_payment_addr: namada_sdk::masp_primitives::sapling::PaymentAddress = viewing_key
            .to_payment_address(div)
            .expect("a PaymentAddress");
        let payment_addr = PaymentAddress::from(masp_payment_addr);

        let payment_address_alias = format!("{}-payment-address", self.source.name);
        wallet.insert_payment_addr(payment_address_alias, payment_addr, true);

        wallet
            .save()
            .map_err(|e| StepError::Wallet(format!("Failed to save the wallet: {e}")))?;
        drop(wallet);

        build_reveal_pk(sdk, sk.to_public()).await
    }

    async fn build_checks(
        &self,
        _sdk: &Sdk,
        _retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        Ok(vec![Check::RevealPk(
            check::reveal_pk::RevealPk::builder()
                .target(self.source.clone())
                .build(),
        )])
    }

    fn update_state(&self, state: &mut State, _with_fee: bool) {
        state.add_implicit_account(&self.source);
        state.add_masp_account(&self.source);
    }
}
