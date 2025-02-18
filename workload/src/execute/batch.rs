use namada_sdk::{signing::SigningTxData, tx::Tx};

use crate::{executor::StepError, sdk::namada::Sdk, task::TaskSettings};

use super::utils::{self, execute_tx};

pub async fn execute_tx_batch(
    sdk: &Sdk,
    txs: Vec<(Tx, SigningTxData)>,
    settings: &TaskSettings,
) -> Result<Option<u64>, StepError> {
    let (tx, signing_datas, tx_args) = utils::merge_tx(sdk, txs, settings)
        .await
        .map_err(|e| StepError::Build(e.to_string()))?;

    execute_tx(sdk, tx, signing_datas, &tx_args).await
}
