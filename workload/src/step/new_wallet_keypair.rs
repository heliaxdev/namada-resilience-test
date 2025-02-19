use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::step::StepContext;
use crate::{
    state::State,
    task::{self, Task},
};

use super::utils;

#[derive(Debug, Default)]
pub struct NewWalletKeyPair;

impl StepContext for NewWalletKeyPair {
    fn name(&self) -> String {
        "new-walleet-keypair".to_string()
    }

    async fn is_valid(&self, _sdk: &Sdk, state: &State) -> Result<bool, StepError> {
        Ok(true)
    }

    async fn build_task(&self, _sdk: &Sdk, state: &mut State) -> Result<Vec<Task>, StepError> {
        let alias = utils::random_alias(state);
        Ok(vec![Task::NewWalletKeyPair(
            task::new_wallet_keypair::NewWalletKeyPair::builder()
                .source(alias)
                .build(),
        )])
    }
}
