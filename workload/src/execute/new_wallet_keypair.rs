use namada_sdk::{
    key::{common, SchemeType},
    masp::find_valid_diversifier,
    masp_primitives::zip32,
    rpc, PaymentAddress,
};
use rand::rngs::OsRng;

use crate::{entities::Alias, sdk::namada::Sdk, steps::StepError};

pub async fn execute_new_wallet_key_pair(
    sdk: &Sdk,
    source_alias: &Alias,
) -> Result<common::PublicKey, StepError> {
    let block = rpc::query_block(&sdk.namada.client)
        .await
        .map_err(|e| StepError::Rpc(e.to_string()))?
        .ok_or_else(|| StepError::StateCheck("No block found".to_string()))?;

    let mut wallet = sdk.namada.wallet.write().await;

    let (_alias, sk) = wallet
        .gen_store_secret_key(
            SchemeType::Ed25519,
            Some(source_alias.name.clone()),
            true,
            None,
            &mut OsRng,
        )
        .ok_or_else(|| StepError::Wallet(format!("Failed to generate keypair")))?;

    let spending_key_alias = format!("{}-spending-key", source_alias.name);
    let (_alias, spending_key) = wallet
        .gen_store_spending_key(
            spending_key_alias.clone(),
            Some(block.height),
            None,
            true,
            &mut OsRng,
        )
        .ok_or_else(|| StepError::Wallet(format!("Failed to generate spending key")))?;

    let viewing_key = zip32::ExtendedFullViewingKey::from(&spending_key.into())
        .fvk
        .vk;
    let (div, _g_d) = find_valid_diversifier(&mut OsRng);
    let masp_payment_addr: namada_sdk::masp_primitives::sapling::PaymentAddress = viewing_key
        .to_payment_address(div)
        .expect("a PaymentAddress");
    let payment_addr = PaymentAddress::from(masp_payment_addr);

    let payment_address_alias = format!("{}-payment-address", source_alias.name);
    wallet.insert_payment_addr(payment_address_alias, payment_addr, true);

    wallet
        .save()
        .map_err(|e| StepError::Wallet(format!("Failed to save the wallet: {e}")))?;

    Ok(sk.to_public())
}
