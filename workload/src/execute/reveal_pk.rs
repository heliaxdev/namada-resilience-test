use namada_sdk::{args::TxBuilder, key::common, tx::data::GasLimit, Namada};

use crate::{constants::DEFAULT_GAS_LIMIT, executor::StepError, sdk::namada::Sdk};

use super::utils::execute_tx;

pub async fn execute_reveal_pk(
    sdk: &Sdk,
    public_key: common::PublicKey,
) -> Result<Option<u64>, StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let fee_payer = wallet
        .find_public_key("faucet")
        .map_err(|e| StepError::Wallet(e.to_string()))?;
    drop(wallet);

    let reveal_pk_tx_builder = sdk
        .namada
        .new_reveal_pk(public_key.clone())
        .signing_keys(vec![public_key])
        .gas_limit(GasLimit::from(DEFAULT_GAS_LIMIT * 2))
        .wrapper_fee_payer(fee_payer);

    let (reveal_tx, signing_data) = reveal_pk_tx_builder
        .build(&sdk.namada)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    execute_tx(sdk, reveal_tx, vec![signing_data], &reveal_pk_tx_builder.tx).await
}
