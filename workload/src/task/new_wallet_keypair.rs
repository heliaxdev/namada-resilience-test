use namada_sdk::args;
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::Tx;
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

impl NewWalletKeyPair {
    pub fn source(&self) -> &Alias {
        &self.source
    }
}

impl TaskContext for NewWalletKeyPair {
    fn name(&self) -> String {
        "new-wallet-keypair".to_string()
    }

    fn summary(&self) -> String {
        format!("new-wallet-keypair/{}", self.source.name)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        None
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;
        let public_key = wallet
            .find_public_key(&self.source.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;
        drop(wallet);

        build_reveal_pk(sdk, public_key).await
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

    fn update_state(&self, state: &mut State) {
        state.add_implicit_account(&self.source);
    }
}
