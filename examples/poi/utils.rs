// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::fs;
use std::fs::File;
use std::path::PathBuf;

use anyhow::{Context, Result, bail, ensure};
use iota_config::{IOTA_GENESIS_FILENAME, iota_config_dir};
use iota_grpc_client::Client as GrpcClient;
use iota_interaction::KeytoolSigner;
use iota_sdk_types::{ObjectId, TransactionDigest};
use iota_types::event::EventID;
use notarization::client::{NotarizationClient, NotarizationClientReadOnly};
use notarization::core::types::{State, TimeLock};
use poi_rs::PoiClient;
use product_common::test_utils::{
    TEST_GAS_BUDGET, get_active_address, get_balance, get_cached_id, get_client, init_product_package, request_funds,
};
use serde::Deserialize;
use tokio::process::Command;

const MAINNET_CHAIN_IDENTIFIER: &str = "6364aad5";
const NOTARIZATION_PACKAGE_ID_ENV: &str = "IOTA_NOTARIZATION_PKG_ID";
const GRPC_URL_ENV: &str = "NETWORK_GRPC_URL";
const GENESIS_PATH_ENV: &str = "IOTA_GENESIS_PATH";
const PUBLISH_SCRIPT_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../notarization-move/scripts/publish_package.sh"
);

/// Clients and network metadata shared by the Rust Proof of Inclusion examples.
pub struct PoiContext {
    /// Active IOTA CLI environment alias.
    pub network_alias: String,
    /// Client used to construct and verify Proof of Inclusion proofs.
    pub poi_client: PoiClient<GrpcClient>,
    chain_identifier: String,
    notarization_client: NotarizationClient<KeytoolSigner>,
}

impl PoiContext {
    /// Returns the network-scoped directory used by persistent PoI example data.
    pub fn poi_dir(&self) -> Result<PathBuf> {
        Ok(iota_config_dir()
            .context("failed to locate the IOTA configuration directory")?
            .join("poi")
            .join(&self.chain_identifier))
    }

    /// Opens the trusted genesis blob for the active network.
    ///
    /// `IOTA_GENESIS_PATH` is required for local and custom networks. For known
    /// public networks, the active chain identifier selects a built-in genesis
    /// URL, and the downloaded blob is cached.
    pub async fn load_genesis(&self) -> Result<File> {
        if let Some(path) = std::env::var_os(GENESIS_PATH_ENV) {
            let path = PathBuf::from(path);
            return File::open(&path).with_context(|| format!("failed to open the genesis blob '{}'", path.display()));
        }

        let (network, url) = match self.chain_identifier.as_str() {
            MAINNET_CHAIN_IDENTIFIER => ("mainnet", "https://dbfiles.mainnet.iota.cafe/genesis.blob"),
            "2304aa97" => ("testnet", "https://dbfiles.testnet.iota.cafe/genesis.blob"),
            "daf90477" => ("devnet", "https://dbfiles.devnet.iota.cafe/genesis.blob"),
            _ => bail!("set {GENESIS_PATH_ENV} to the trusted genesis blob for the active network"),
        };
        let path = self.poi_dir()?.join(IOTA_GENESIS_FILENAME);

        if !path.is_file() {
            let parent = path.parent().context("the genesis cache path must have a parent")?;
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create genesis cache directory '{}'", parent.display()))?;

            println!("Downloading the trusted {network} genesis blob...");
            let bytes = reqwest::get(url)
                .await
                .with_context(|| format!("failed to download the {network} genesis blob"))?
                .error_for_status()
                .with_context(|| format!("the {network} genesis download returned an error"))?
                .bytes()
                .await
                .with_context(|| format!("failed to read the downloaded {network} genesis blob"))?;
            fs::write(&path, bytes)
                .with_context(|| format!("failed to cache the genesis blob at '{}'", path.display()))?;
        }

        File::open(&path).with_context(|| format!("failed to open the genesis blob '{}'", path.display()))
    }

    /// Creates a fresh locked `Notarization` object and returns its proof targets.
    pub async fn create_notarization(&self, label: &str) -> Result<NotarizationTargets> {
        let created = self
            .notarization_client
            .create_locked_notarization()
            .with_state(State::from_string(label.to_owned(), None))
            .with_delete_lock(TimeLock::None)
            .finish()
            .context("failed to finish the locked Notarization builder")?
            .build_and_execute(&self.notarization_client)
            .await
            .context("failed to create the example Notarization object")?;

        let transaction_digest = created.response.digest;
        let object_id = *created.output.id.object_id();
        let event_id = created
            .response
            .events
            .as_ref()
            .context("the setup transaction response did not include events")?
            .data
            .first()
            .context("the setup transaction did not emit LockedNotarizationCreated")?
            .id;

        ensure!(
            event_id.tx_digest == transaction_digest,
            "the creation event does not belong to the setup transaction"
        );

        Ok(NotarizationTargets {
            transaction_digest,
            object_id,
            event_id,
        })
    }
}

/// Fresh transaction, object, and event targets produced for an example.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotarizationTargets {
    /// Transaction that created the `Notarization` object.
    pub transaction_digest: TransactionDigest,
    /// Locked `Notarization` object created by the transaction.
    pub object_id: ObjectId,
    /// `LockedNotarizationCreated` event emitted by the transaction.
    pub event_id: EventID,
}

/// Configures the Rust examples from the active IOTA CLI environment and wallet.
///
/// `IOTA_NOTARIZATION_PKG_ID` overrides package discovery. When it is absent on
/// a non-mainnet network, the Single Notarization Move Package is published once
/// and cached by chain identifier. Mainnet always requires an explicit package ID.
pub async fn prepare_poi_example() -> Result<PoiContext> {
    let cli_environment = active_cli_environment().await?;
    let rpc_url = cli_environment.rpc.clone();
    let iota_client = get_client(&rpc_url)
        .await
        .with_context(|| format!("failed to connect to the active CLI environment at {rpc_url}"))?;
    let chain_identifier = iota_client
        .read_api()
        .get_chain_identifier()
        .await
        .context("failed to read the active network's chain identifier")?;
    let is_mainnet = cli_environment.alias == "mainnet" || chain_identifier == MAINNET_CHAIN_IDENTIFIER;
    let grpc_url = grpc_url(&cli_environment, &chain_identifier)?;
    let poi_client = PoiClient::from_grpc_client(
        GrpcClient::new(&grpc_url).with_context(|| format!("invalid gRPC URL '{grpc_url}'"))?,
    );

    let sender_address = get_active_address()
        .await
        .context("failed to read the active IOTA CLI wallet address")?;
    let signer = KeytoolSigner::builder()
        .with_address(sender_address)
        .build()
        .context("failed to load the active IOTA CLI wallet signer")?;
    let configured_package_id = configured_package_id()?;

    if is_mainnet && configured_package_id.is_none() {
        bail!(
            "{NOTARIZATION_PACKAGE_ID_ENV} must be set on mainnet; the examples never publish a package automatically on mainnet"
        );
    }

    let package_cache_directory = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/poi-examples");
    fs::create_dir_all(package_cache_directory).context("failed to create the package cache directory")?;
    let package_cache_file = format!("{package_cache_directory}/notarization-package-{chain_identifier}.txt");
    let cached_package_id = get_cached_id(&chain_identifier, Some(&package_cache_file))
        .await
        .ok()
        .and_then(|package_id| package_id.parse().ok());
    let cached_or_configured_package_id = configured_package_id.or(cached_package_id);

    let required_balance = TEST_GAS_BUDGET * 2
        + if cached_or_configured_package_id.is_some() {
            0
        } else {
            500_000_000
        };
    let balance = get_balance(sender_address)
        .await
        .context("failed to read the active IOTA CLI wallet balance")?;
    if balance < required_balance {
        if is_mainnet {
            bail!(
                "the active IOTA CLI wallet {sender_address} has {balance} nanos but the examples require at least {required_balance}; fund the wallet before running the examples"
            );
        }
        request_funds(&sender_address)
            .await
            .context("failed to fund the active IOTA CLI wallet from the configured faucet")?;
    }

    let package_id = match cached_or_configured_package_id {
        Some(package_id) => package_id,
        None => {
            let package_id = init_product_package(&iota_client, Some(&package_cache_file), Some(PUBLISH_SCRIPT_FILE))
                .await
                .context("failed to initialize the Single Notarization Move Package")?;

            // product-core v0.8.24 writes the active address here, while its
            // cache reader expects the chain identifier.
            fs::write(&package_cache_file, format!("{package_id};{chain_identifier}"))
                .context("failed to correct the package cache")?;
            package_id
        }
    };
    let read_only_client = NotarizationClientReadOnly::new_with_pkg_id(iota_client, package_id)
        .await
        .context("failed to create the read-only Single Notarization client")?;
    let notarization_client = NotarizationClient::new(read_only_client, signer)
        .await
        .context("failed to attach the active IOTA CLI wallet signer")?;

    Ok(PoiContext {
        network_alias: cli_environment.alias,
        poi_client,
        chain_identifier,
        notarization_client,
    })
}

#[derive(Debug, Deserialize)]
struct CliEnvironment {
    alias: String,
    rpc: String,
    grpc: Option<String>,
}

async fn active_cli_environment() -> Result<CliEnvironment> {
    let output = Command::new("iota")
        .args(["client", "envs", "--json"])
        .output()
        .await
        .context("failed to execute `iota client envs`")?;
    if !output.status.success() {
        bail!(
            "failed to read the configured IOTA CLI environments: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let (environments, active_alias): (Vec<CliEnvironment>, String) =
        serde_json::from_slice(&output.stdout).context("failed to parse `iota client envs --json`")?;

    environments
        .into_iter()
        .find(|environment| environment.alias == active_alias)
        .with_context(|| format!("active IOTA CLI environment '{active_alias}' was not found"))
}

fn grpc_url(environment: &CliEnvironment, chain_identifier: &str) -> Result<String> {
    if let Some(url) = std::env::var_os(GRPC_URL_ENV) {
        return url
            .into_string()
            .map_err(|_| anyhow::anyhow!("{GRPC_URL_ENV} is not valid UTF-8"));
    }
    if let Some(url) = &environment.grpc {
        return Ok(url.clone());
    }

    let url = match chain_identifier {
        MAINNET_CHAIN_IDENTIFIER => "https://grpc.mainnet.iota.cafe:443",
        "2304aa97" => "https://grpc.testnet.iota.cafe:443",
        "daf90477" => "https://grpc.devnet.iota.cafe:443",
        _ if environment.alias == "localnet" => return Ok("http://127.0.0.1:50051".to_owned()),
        _ => {
            bail!(
                "the active CLI environment '{}' for chain {chain_identifier} has no gRPC URL; set {GRPC_URL_ENV}",
                environment.alias,
            )
        }
    };

    Ok(url.to_owned())
}

fn configured_package_id() -> Result<Option<ObjectId>> {
    let Some(value) = std::env::var_os(NOTARIZATION_PACKAGE_ID_ENV) else {
        return Ok(None);
    };
    let value = value
        .into_string()
        .map_err(|_| anyhow::anyhow!("{NOTARIZATION_PACKAGE_ID_ENV} is not valid UTF-8"))?;
    value
        .parse()
        .with_context(|| format!("{NOTARIZATION_PACKAGE_ID_ENV} contains an invalid package ID"))
        .map(Some)
}
