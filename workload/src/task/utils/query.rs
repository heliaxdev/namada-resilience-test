use std::{
    str::FromStr,
    time::{Duration, Instant},
};

use namada_sdk::{
    address::Address,
    control_flow::install_shutdown_signal,
    io::DevNullProgressBar,
    masp::{
        shielded_wallet::ShieldedApi, IndexerMaspClient, LedgerMaspClient, MaspLocalTaskEnv,
        ShieldedSyncConfig,
    },
    masp_primitives::zip32,
    rpc, token, Namada,
};
use namada_wallet::DatedKeypair;
use reqwest::Url;
use serde_json::json;
use tryhard::{backoff_strategies::ExponentialBackoff, NoOnRetry, RetryFutureConfig};

use crate::{entities::Alias, executor::StepError, sdk::namada::Sdk};

pub async fn get_balance(
    sdk: &Sdk,
    source: &Alias,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<token::Amount, StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let native_token_alias = Alias::nam();
    let native_token_address = wallet
        .find_address(&native_token_alias.name)
        .ok_or_else(|| {
            StepError::Wallet(format!(
                "No native token address: {}",
                native_token_alias.name
            ))
        })?;
    let target_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| StepError::Wallet(format!("No target address: {}", source.name)))?;

    tryhard::retry_fn(|| {
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
    .map_err(|e| StepError::BuildCheck(e.to_string()))
}

pub async fn get_shielded_balance(
    sdk: &Sdk,
    source: &Alias,
    height: Option<u64>,
    with_indexer: bool,
) -> Result<Option<token::Amount>, StepError> {
    let (is_successful, error) = match shielded_sync(sdk, height, with_indexer).await {
        Ok(_) => (true, "".to_string()),
        Err(e) => (false, e.to_string()),
    };

    tracing::warn!("First shielded sync result: {is_successful}, err: {error}");

    if with_indexer {
        antithesis_sdk::assert_sometimes!(
            is_successful,
            "shielded sync (indexer) was successful.",
            &json!({
                "source": source,
                "error": error
            })
        );
    } else {
        antithesis_sdk::assert_always_or_unreachable!(
            is_successful,
            "shielded sync (node) was successful.",
            &json!({
                "source": source,
                "error": error
            })
        );
    }

    if with_indexer && !is_successful {
        let (is_successful, error) = match shielded_sync(sdk, height, false).await {
            Ok(_) => (true, "".to_string()),
            Err(e) => (false, e.to_string()),
        };

        tracing::warn!("Second shielded sync result: {is_successful}, err: {error}");

        antithesis_sdk::assert_always_or_unreachable!(
            is_successful,
            "shielded sync (node) was successful.",
            &json!({
                "source": source,
                "error": error
            })
        );
    }

    let client = &sdk.namada.client;

    let masp_epoch = rpc::query_masp_epoch(client)
        .await
        .map_err(StepError::Rpc)?;
    let native_token = rpc::query_native_token(client)
        .await
        .map_err(StepError::Rpc)?;

    let mut wallet = sdk.namada.wallet.write().await;
    let spending_key = if source.name.ends_with("-spending-key") {
        source.name.clone()
    } else {
        format!(
            "{}-spending-key",
            source
                .name
                .strip_suffix("-payment-address")
                .unwrap_or(&source.name)
        )
    };
    let target_spending_key = wallet
        .find_spending_key(&spending_key, None)
        .map_err(|e| StepError::Wallet(e.to_string()))?
        .to_owned();
    drop(wallet);

    let mut shielded_ctx = sdk.namada.shielded_mut().await;

    let viewing_key = zip32::ExtendedFullViewingKey::from(&target_spending_key.into())
        .fvk
        .vk;

    let Some(balance) = shielded_ctx
        .compute_shielded_balance(&viewing_key)
        .await
        .map_err(|e| StepError::BuildCheck(e.to_string()))?
    else {
        return Ok(None);
    };

    let total_balance = shielded_ctx
        .decode_combine_sum_to_epoch(client, balance, masp_epoch)
        .await
        .0
        .get(&native_token);

    Ok(Some(total_balance.into()))
}

pub async fn get_bond(
    sdk: &Sdk,
    source: &Alias,
    validator: &str,
    epoch: u64,
    retry_config: RetryFutureConfig<ExponentialBackoff, NoOnRetry>,
) -> Result<token::Amount, StepError> {
    let wallet = sdk.namada.wallet.read().await;
    let source_address = wallet
        .find_address(&source.name)
        .ok_or_else(|| StepError::Wallet(format!("No source address: {}", source.name)))?;
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
    .map_err(|e| StepError::BuildCheck(e.to_string()))
}

pub async fn shielded_sync(
    sdk: &Sdk,
    height: Option<u64>,
    with_indexer: bool,
) -> Result<(), StepError> {
    let now = Instant::now();
    tracing::info!("Started shielded sync (using indexer: {})...", with_indexer);

    let wallet = sdk.namada.wallet.read().await;
    let vks = sdk
        .namada
        .wallet()
        .await
        .get_viewing_keys()
        .values()
        .map(|vk| DatedKeypair::new(vk.as_viewing_key(), None))
        .collect::<Vec<_>>();
    drop(wallet);

    let mut shielded_ctx = sdk.namada.shielded_mut().await;

    let task_env = MaspLocalTaskEnv::new(4).map_err(|e| StepError::ShieldedSync(e.to_string()))?;
    let shutdown_signal = install_shutdown_signal(true);
    let enable_wait = height.is_some();
    let height = height.map(|h| h.into());
    tracing::info!("Using height with shielded sync: {height:?}");

    let res = if with_indexer {
        let masp_client = IndexerMaspClient::new(
            reqwest::Client::new(),
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
            .map_err(|e| StepError::ShieldedSync(e.to_string()))
    } else {
        let masp_client =
            LedgerMaspClient::new(sdk.namada.clone_client(), 10, Duration::from_secs(1));

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
            .map_err(|e| StepError::ShieldedSync(e.to_string()))
    };

    shielded_ctx
        .save()
        .await
        .map_err(|e| StepError::ShieldedSync(e.to_string()))?;

    tracing::info!(
        "Done shielded sync (took {}s, with indexer: {with_indexer})!",
        now.elapsed().as_secs(),
    );

    res
}
