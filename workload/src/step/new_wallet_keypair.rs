use namada_sdk::key::SchemeType;
use namada_sdk::masp::find_valid_diversifier;
use namada_sdk::masp_primitives::zip32;
use namada_sdk::{rpc, PaymentAddress};
use rand::rngs::OsRng;
use serde_json::json;

use crate::code::Code;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::step::StepContext;
use crate::task::{self, Task};
use crate::{assert_always_step, assert_sometimes_step, assert_unrechable_step};

use super::utils;

#[derive(Clone, Debug, Default)]
pub struct NewWalletKeyPair;

impl StepContext for NewWalletKeyPair {
    fn name(&self) -> String {
        "new-wallet-keypair".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, _state: &State) -> Result<bool, StepError> {
        Ok(true)
    }

    async fn build_task(&self, sdk: &Sdk, _state: &State) -> Result<Vec<Task>, StepError> {
        let alias = utils::random_alias();

        let block = rpc::query_block(&sdk.namada.client)
            .await
            .map_err(StepError::Rpc)?
            .ok_or_else(|| StepError::StateCheck("No block found".to_string()))?;

        let mut wallet = sdk.namada.wallet.write().await;

        wallet
            .gen_store_secret_key(
                SchemeType::Ed25519,
                Some(alias.name.clone()),
                true,
                None,
                &mut OsRng,
            )
            .ok_or_else(|| {
                StepError::Wallet(format!("Failed to generate keypair for {}", alias.name))
            })?;

        let spending_key_alias = alias.spending_key().name;
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
            .expect("Conversion to PaymentAddress shouldn't fail");
        let payment_addr = PaymentAddress::from(masp_payment_addr);

        let payment_address_alias = alias.payment_address().name;
        wallet
            .insert_payment_addr(payment_address_alias.clone(), payment_addr, true)
            .ok_or_else(|| {
                StepError::Wallet(format!(
                    "Failed to insert payment address for {}",
                    payment_address_alias
                ))
            })?;

        wallet
            .save()
            .map_err(|e| StepError::Wallet(format!("Failed to save the wallet: {e}")))?;
        drop(wallet);

        Ok(vec![Task::NewWalletKeyPair(
            task::new_wallet_keypair::NewWalletKeyPair::builder()
                .source(alias)
                .build(),
        )])
    }

    fn assert(&self, code: &Code) {
        let is_fatal = code.is_fatal();
        let is_successful = code.is_successful();

        let details = json!({"outcome": code.code()});

        if is_fatal {
            assert_unrechable_step!("Fatal NewWalletKeyPair", details)
        } else if is_successful {
            assert_always_step!("Done NewWalletKeyPair", details)
        } else {
            assert_sometimes_step!("Failed NewWalletKeyPair ", details)
        }
    }
}
