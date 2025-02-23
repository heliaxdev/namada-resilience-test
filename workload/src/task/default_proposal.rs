use std::collections::BTreeMap;

use namada_sdk::args::{self, TxBuilder};
use namada_sdk::governance::cli::onchain::{DefaultProposal as Proposal, OnChainProposal};
use namada_sdk::signing::SigningTxData;
use namada_sdk::tx::data::GasLimit;
use namada_sdk::tx::Tx;
use namada_sdk::Namada;
use typed_builder::TypedBuilder;

use crate::check::{self, Check};
use crate::constants::PROPOSAL_DEPOSIT;
use crate::executor::StepError;
use crate::sdk::namada::Sdk;
use crate::state::State;
use crate::task::{TaskContext, TaskSettings};
use crate::types::{Alias, Epoch};
use crate::utils::{get_balance, RetryConfig};

#[derive(Clone, Debug, TypedBuilder)]
pub struct DefaultProposal {
    source: Alias,
    start_epoch: Epoch,
    end_epoch: Epoch,
    grace_epoch: Epoch,
    settings: TaskSettings,
}

impl TaskContext for DefaultProposal {
    fn name(&self) -> String {
        "default-proposal".to_string()
    }

    fn summary(&self) -> String {
        format!("default-proposal/{}", self.source.name)
    }

    fn task_settings(&self) -> Option<&TaskSettings> {
        Some(&self.settings)
    }

    async fn build_tx(&self, sdk: &Sdk) -> Result<(Tx, Vec<SigningTxData>, args::Tx), StepError> {
        let wallet = sdk.namada.wallet.read().await;
        let source_address = wallet
            .find_address(&self.source.name)
            .ok_or_else(|| StepError::Wallet(format!("No source address: {}", self.source.name)))?;
        let fee_payer = wallet
            .find_public_key(&self.settings.gas_payer.name)
            .map_err(|e| StepError::Wallet(e.to_string()))?;

        let default_proposal = Proposal {
            proposal: OnChainProposal {
                content: BTreeMap::from_iter([("workload".to_string(), "tester".to_string())]),
                author: source_address.into_owned(),
                voting_start_epoch: self.start_epoch.into(),
                voting_end_epoch: self.end_epoch.into(),
                activation_epoch: self.grace_epoch.into(),
            },
            data: if self.start_epoch % 2 == 0 {
                Some(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
            } else {
                None
            },
        };
        let proposal_json =
            serde_json::to_string(&default_proposal).expect("Encoding proposal shouldn't fail");

        let mut default_proposal_tx_builder =
            sdk.namada.new_init_proposal(proposal_json.into_bytes());

        default_proposal_tx_builder =
            default_proposal_tx_builder.gas_limit(GasLimit::from(self.settings.gas_limit));
        default_proposal_tx_builder = default_proposal_tx_builder.wrapper_fee_payer(fee_payer);

        let mut signing_keys = vec![];
        for signer in &self.settings.signers {
            let public_key = wallet
                .find_public_key(&signer.name)
                .map_err(|e| StepError::Wallet(e.to_string()))?;
            signing_keys.push(public_key)
        }
        default_proposal_tx_builder = default_proposal_tx_builder.signing_keys(signing_keys);
        drop(wallet);

        let (default_proposal, signing_data) = default_proposal_tx_builder
            .build(&sdk.namada)
            .await
            .map_err(|e| StepError::BuildTx(e.to_string()))?;

        Ok((
            default_proposal,
            vec![signing_data],
            default_proposal_tx_builder.tx,
        ))
    }

    async fn build_checks(
        &self,
        sdk: &Sdk,
        retry_config: RetryConfig,
    ) -> Result<Vec<Check>, StepError> {
        let (_, pre_balance) = get_balance(sdk, &self.source, retry_config).await?;

        Ok(vec![Check::BalanceSource(
            check::balance_source::BalanceSource::builder()
                .target(self.source.clone())
                .pre_balance(pre_balance)
                .amount(PROPOSAL_DEPOSIT)
                .build(),
        )])
    }

    fn update_state(&self, state: &mut State, with_fee: bool) {
        if with_fee {
            state.modify_balance_fee(&self.settings.gas_payer, self.settings.gas_limit);
        }
        state.decrease_balance(&self.source, PROPOSAL_DEPOSIT);
        state.add_proposal(self.start_epoch, self.end_epoch);
    }
}
