use std::collections::HashMap;
use std::str::FromStr;
use std::time::{self, Instant};

use namada_sdk::account::Account;
use namada_sdk::address::Address;
use namada_sdk::control_flow::install_shutdown_signal;
use namada_sdk::io::DevNullProgressBar;
use namada_sdk::masp::shielded_wallet::ShieldedApi;
use namada_sdk::masp::{IndexerMaspClient, LedgerMaspClient, MaspLocalTaskEnv, ShieldedSyncConfig};
use namada_sdk::masp_primitives::zip32;
use namada_sdk::proof_of_stake::types::ValidatorStateInfo;
use namada_sdk::token::{self, MaspEpoch};
use namada_sdk::{rpc, Namada};
use namada_wallet::DatedKeypair;
use reqwest::Url;
use serde_json::json;
use tokio::time::{sleep, Duration};
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::error::QueryError;
use crate::sdk::namada::Sdk;
use crate::types::{Alias, Epoch, Height, ProposalId, ProposalVote};
use crate::utils::RetryConfig;

pub async fn get_account_info(
    sdk: &Sdk,
    source: &Alias,
    retry_config: RetryConfig,
) -> Result<(Address, Option<Account>), QueryError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| QueryError::Wallet(format!("No account address: {}", source.name)))?
        .into_owned();
    drop(wallet);

    let account = tryhard::retry_fn(|| rpc::get_account_info(&sdk.namada.client, &source_address))
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {} due to {}...", attempt, error);
            }
        })
        .await
        .map_err(QueryError::Rpc)?;

    Ok((source_address, account))
}

pub async fn is_validator(
    sdk: &Sdk,
    target: &Alias,
    retry_config: RetryConfig,
) -> Result<(Address, bool), QueryError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet
        .find_address(&target.name)
        .ok_or_else(|| QueryError::Wallet(format!("No target address: {}", target.name)))?
        .into_owned();
    drop(wallet);

    let is_validator = tryhard::retry_fn(|| rpc::is_validator(&sdk.namada.client, &source_address))
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {} due to {}...", attempt, error);
            }
        })
        .await
        .map_err(QueryError::Rpc)?;

    Ok((source_address, is_validator))
}

pub async fn get_validator_state(
    sdk: &Sdk,
    target: &Alias,
    epoch: Epoch,
    retry_config: RetryConfig,
) -> Result<(Address, ValidatorStateInfo), QueryError> {
    let wallet = sdk.namada.wallet.read().await;
    let target_address = wallet
        .find_address(&target.name)
        .ok_or_else(|| QueryError::Wallet(format!("No target address: {}", target.name)))?
        .into_owned();
    drop(wallet);

    let state = tryhard::retry_fn(|| {
        rpc::get_validator_state(&sdk.namada.client, &target_address, Some(epoch.into()))
    })
    .with_config(retry_config)
    .on_retry(|attempt, _, error| {
        let error = error.to_string();
        async move {
            tracing::info!("Retry {attempt} due to {error}...");
        }
    })
    .await
    .map_err(QueryError::Rpc)?;

    Ok((target_address, state))
}

pub async fn get_validator_addresses(
    sdk: &Sdk,
    retry_config: RetryConfig,
) -> Result<Vec<Address>, QueryError> {
    let current_epoch = get_epoch(sdk, retry_config).await?;
    let validators = rpc::get_all_consensus_validators(&sdk.namada.client, current_epoch.into())
        .await
        .map_err(QueryError::Rpc)?
        .into_iter()
        .map(|v| v.address)
        .collect();

    Ok(validators)
}

pub async fn is_pk_revealed(
    sdk: &Sdk,
    target: &Alias,
    retry_config: RetryConfig,
) -> Result<bool, QueryError> {
    let wallet = sdk.namada.wallet.read().await;
    let target_address = wallet
        .find_address(&target.name)
        .ok_or_else(|| QueryError::Wallet(format!("No target address: {}", target.name)))?
        .into_owned();
    drop(wallet);

    tryhard::retry_fn(|| rpc::is_public_key_revealed(&sdk.namada.client, &target_address))
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {attempt} due to {error}...");
            }
        })
        .await
        .map_err(QueryError::Rpc)
}

pub async fn get_balance(
    sdk: &Sdk,
    source: &Alias,
    retry_config: RetryConfig,
) -> Result<(Address, token::Amount), QueryError> {
    let wallet = sdk.namada.wallet.read().await;
    let native_token_alias = Alias::nam();
    let native_token_address = wallet
        .find_address(&native_token_alias.name)
        .ok_or_else(|| {
            QueryError::Wallet(format!(
                "No native token address: {}",
                native_token_alias.name
            ))
        })?;
    let target_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| QueryError::Wallet(format!("No target address: {}", source.name)))?;

    let balance = tryhard::retry_fn(|| {
        rpc::get_token_balance(
            &sdk.namada.client,
            &native_token_address,
            &target_address,
            None,
        )
    })
    .with_config(retry_config)
    .on_retry(|attempt, _, error| {
        let error = error.to_string();
        async move {
            tracing::info!("Retry {attempt} due to {error}...");
        }
    })
    .await
    .map_err(QueryError::Rpc)?;

    Ok((target_address.into_owned(), balance))
}

/// Shielded balance. Need shielded-sync in advance.
pub async fn get_shielded_balance(
    sdk: &Sdk,
    source: &Alias,
    retry_config: RetryConfig,
) -> Result<Option<token::Amount>, QueryError> {
    let client = &sdk.namada.client;

    let masp_epoch = get_masp_epoch(sdk, retry_config).await?;

    let mut wallet = sdk.namada.wallet.write().await;
    let native_token_alias = Alias::nam();
    let native_token_address = wallet
        .find_address(&native_token_alias.name)
        .ok_or_else(|| {
            QueryError::Wallet(format!(
                "No native token address: {}",
                native_token_alias.name
            ))
        })?
        .into_owned();
    let spending_key = source.spending_key().name;
    let target_spending_key = wallet
        .find_spending_key(&spending_key, None)
        .map_err(|e| QueryError::Wallet(e.to_string()))?
        .to_owned();
    drop(wallet);

    let mut shielded_ctx = sdk.namada.shielded_mut().await;

    let viewing_key = zip32::ExtendedFullViewingKey::from(&target_spending_key.into())
        .fvk
        .vk;

    let Some(balance) = shielded_ctx
        .compute_shielded_balance(&viewing_key)
        .await
        .map_err(|e| QueryError::ShieldedContext(e.to_string()))?
    else {
        return Ok(None);
    };

    let total_balance = shielded_ctx
        .decode_combine_sum_to_epoch(client, balance, masp_epoch)
        .await
        .0
        .get(&native_token_address);

    Ok(Some(total_balance.into()))
}

pub async fn get_block_height(sdk: &Sdk, retry_config: RetryConfig) -> Result<Height, QueryError> {
    let block = tryhard::retry_fn(|| rpc::query_block(&sdk.namada.client))
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {} due to {}...", attempt, error);
            }
        })
        .await
        .map_err(QueryError::Rpc)?
        .expect("Block should exist");
    Ok(block.height.into())
}

pub async fn wait_block_settlement(sdk: &Sdk, height: Height, retry_config: RetryConfig) {
    loop {
        if let Ok(current_height) = get_block_height(sdk, retry_config).await {
            if current_height > height {
                break;
            } else {
                tracing::info!(
                    "Waiting for block settlement at height: {}, currently at: {}",
                    height,
                    current_height
                );
            }
        }
        sleep(Duration::from_secs(2)).await
    }
}

pub async fn get_epoch(sdk: &Sdk, retry_config: RetryConfig) -> Result<Epoch, QueryError> {
    tryhard::retry_fn(|| rpc::query_epoch(&sdk.namada.client))
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {} due to {}...", attempt, error);
            }
        })
        .await
        .map(|epoch| epoch.into())
        .map_err(QueryError::Rpc)
}

pub async fn get_masp_epoch(sdk: &Sdk, retry_config: RetryConfig) -> Result<MaspEpoch, QueryError> {
    tryhard::retry_fn(|| rpc::query_masp_epoch(&sdk.namada.client))
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {} due to {}...", attempt, error);
            }
        })
        .await
        .map_err(QueryError::Rpc)
}

pub async fn get_masp_epoch_at_height(
    sdk: &Sdk,
    height: Height,
    retry_config: RetryConfig,
) -> Result<MaspEpoch, QueryError> {
    let epoch = tryhard::retry_fn(|| rpc::query_epoch_at_height(&sdk.namada.client, height.into()))
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {} due to {}...", attempt, error);
            }
        })
        .await
        .map_err(QueryError::Rpc)?
        .expect("Epoch should exist");
    let key = namada_sdk::parameters::storage::get_masp_epoch_multiplier_key();
    let masp_epoch_multiplier =
        tryhard::retry_fn(|| rpc::query_storage_value(&sdk.namada.client, &key))
            .with_config(retry_config)
            .on_retry(|attempt, _, error| {
                let error = error.to_string();
                async move {
                    tracing::info!("Retry {} due to {}...", attempt, error);
                }
            })
            .await
            .map_err(QueryError::Rpc)?;

    MaspEpoch::try_from_epoch(epoch, masp_epoch_multiplier)
        .map_err(|e| QueryError::Convert(e.to_string()))
}

pub async fn get_bond(
    sdk: &Sdk,
    source: &Alias,
    validator: &str,
    epoch: Epoch,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<token::Amount, QueryError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| QueryError::Wallet(format!("No source address: {}", source.name)))?;
    let validator_address =
        Address::from_str(validator).expect("ValidatorAddress should be converted");
    let epoch = namada_sdk::state::Epoch::from(epoch);

    tryhard::retry_fn(|| {
        rpc::get_bond_amount_at(
            &sdk.namada.client,
            &source_address,
            &validator_address,
            epoch.next().next(),
        )
    })
    .with_config(retry_config)
    .on_retry(|attempt, _, error| {
        let error = error.to_string();
        async move {
            tracing::info!("Retry {} due to {}...", attempt, error);
        }
    })
    .await
    .map_err(QueryError::Rpc)
}

pub async fn get_rewards(
    sdk: &Sdk,
    source: &Alias,
    validator: &str,
    epoch: Epoch,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<token::Amount, QueryError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| QueryError::Wallet(format!("No source address: {}", source.name)))?;
    let source = Some(source_address.into_owned());
    let validator_address =
        Address::from_str(validator).expect("ValidatorAddress should be converted");
    let epoch = Some(namada_sdk::state::Epoch::from(epoch));

    tryhard::retry_fn(|| {
        rpc::query_rewards(&sdk.namada.client, &source, &validator_address, &epoch)
    })
    .with_config(retry_config)
    .on_retry(|attempt, _, error| {
        let error = error.to_string();
        async move {
            tracing::info!("Retry {} due to {}...", attempt, error);
        }
    })
    .await
    .map_err(QueryError::Rpc)
}

pub async fn shielded_sync_with_retry(
    sdk: &Sdk,
    source: &Alias,
    height: Option<Height>,
    with_indexer: bool,
) -> Result<(), QueryError> {
    let (is_successful, error) = match shielded_sync(sdk, height, with_indexer).await {
        Ok(_) => (true, "".to_string()),
        Err(e) => (false, e.to_string()),
    };

    tracing::warn!("First shielded sync result: {is_successful}, err: {error}");

    if with_indexer {
        antithesis_sdk::assert_sometimes!(
            is_successful,
            "shielded sync (indexer) was successful",
            &json!({
                "source": source,
                "error": error
            })
        );
    } else {
        antithesis_sdk::assert_always_or_unreachable!(
            is_successful,
            "shielded sync (node) was successful",
            &json!({
                "source": source,
                "error": error
            })
        );
    }

    if is_successful {
        return Ok(());
    } else if !with_indexer {
        return Err(QueryError::ShieldedSync(error));
    }

    // Try shielded sync without indexer only if the shielded sync with indexer failed
    let (is_successful, error) = match shielded_sync(sdk, height, false).await {
        Ok(_) => (true, "".to_string()),
        Err(e) => (false, e.to_string()),
    };

    tracing::warn!("Second shielded sync result: {is_successful}, err: {error}");

    antithesis_sdk::assert_always_or_unreachable!(
        is_successful,
        "Second shielded sync (node) was successful",
        &json!({
            "source": source,
            "error": error
        })
    );

    if is_successful {
        Ok(())
    } else {
        Err(QueryError::ShieldedSync(error))
    }
}

async fn shielded_sync(
    sdk: &Sdk,
    height: Option<Height>,
    with_indexer: bool,
) -> Result<(), QueryError> {
    let now = Instant::now();
    tracing::info!("Started shielded sync (using indexer: {})...", with_indexer);

    let wallet = sdk.namada.wallet.read().await;
    let vks = wallet
        .get_viewing_keys()
        .iter()
        .map(|(alias, vk)| {
            let birthday = wallet.find_birthday(alias).cloned();
            DatedKeypair::new(vk.as_viewing_key(), birthday)
        })
        .collect::<Vec<_>>();
    drop(wallet);

    let mut shielded_ctx = sdk.namada.shielded_mut().await;

    let task_env = MaspLocalTaskEnv::new(4).map_err(|e| QueryError::ShieldedSync(e.to_string()))?;
    let shutdown_signal = install_shutdown_signal(true);
    let enable_wait = height.is_some();
    let height = height.map(|h| h.into());
    tracing::info!("Using height with shielded sync: {height:?}");

    let res = if with_indexer {
        let client = reqwest::Client::builder()
            .connect_timeout(time::Duration::from_secs(60))
            .build()
            .expect("Client should be built");
        let masp_client = IndexerMaspClient::new(
            client,
            Url::parse(&sdk.masp_indexer_url).unwrap(),
            false,
            20,
        );

        let config = ShieldedSyncConfig::builder()
            .client(masp_client)
            .fetched_tracker(DevNullProgressBar)
            .scanned_tracker(DevNullProgressBar)
            .applied_tracker(DevNullProgressBar)
            .shutdown_signal(shutdown_signal)
            .wait_for_last_query_height(enable_wait)
            .build();

        shielded_ctx
            .sync(task_env, config, height, &[], &vks)
            .await
            .map_err(|e| QueryError::ShieldedSync(e.to_string()))
    } else {
        let masp_client =
            LedgerMaspClient::new(sdk.namada.clone_client(), 10, time::Duration::from_secs(1));

        let config = ShieldedSyncConfig::builder()
            .client(masp_client)
            .fetched_tracker(DevNullProgressBar)
            .scanned_tracker(DevNullProgressBar)
            .applied_tracker(DevNullProgressBar)
            .shutdown_signal(shutdown_signal)
            .wait_for_last_query_height(enable_wait)
            .build();

        shielded_ctx
            .sync(task_env, config, height, &[], &vks)
            .await
            .map_err(|e| QueryError::ShieldedSync(e.to_string()))
    };

    if res.is_err() {
        // revert the shielded context
        shielded_ctx
            .load()
            .await
            .map_err(|e| QueryError::ShieldedSync(e.to_string()))?;
    }

    tracing::info!(
        "Done shielded sync (took {}s, with indexer: {with_indexer})!",
        now.elapsed().as_secs(),
    );

    res
}

pub async fn get_proposals(
    sdk: &Sdk,
    last_proposal_id: Option<ProposalId>,
) -> Result<HashMap<ProposalId, (Epoch, Epoch)>, QueryError> {
    let mut proposals = HashMap::new();
    let mut proposal_id = last_proposal_id.map(|id| id + 1).unwrap_or_default();
    while let Some(proposal) = rpc::query_proposal_by_id(&sdk.namada.client, proposal_id)
        .await
        .map_err(QueryError::Rpc)?
    {
        proposals.insert(
            proposal_id,
            (proposal.voting_start_epoch.0, proposal.voting_end_epoch.0),
        );

        proposal_id += 1;
    }

    Ok(proposals)
}

pub async fn get_vote_results(
    sdk: &Sdk,
    target: &Alias,
    proposal_id: ProposalId,
    retry_config: RetryConfig,
) -> Result<Vec<ProposalVote>, QueryError> {
    let wallet = sdk.namada.wallet.read().await;
    let target_address = wallet
        .find_address(&target.name)
        .ok_or_else(|| QueryError::Wallet(format!("No target address: {}", target.name)))?
        .into_owned();
    drop(wallet);

    let votes = tryhard::retry_fn(|| rpc::query_proposal_votes(&sdk.namada.client, proposal_id))
        .with_config(retry_config)
        .on_retry(|attempt, _, error| {
            let error = error.to_string();
            async move {
                tracing::info!("Retry {} due to {}...", attempt, error);
            }
        })
        .await
        .map_err(QueryError::Rpc)?;

    let votes = votes
        .into_iter()
        .filter_map(|vote| {
            if vote.delegator == target_address {
                Some(vote.data)
            } else {
                None
            }
        })
        .collect();

    Ok(votes)
}
