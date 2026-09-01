// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
use iota_config::{IOTA_GENESIS_FILENAME, genesis::Genesis, iota_config_dir};
use iota_grpc_client::Client as GrpcClient;
use iota_sdk_types::{ObjectId, TransactionDigest};
use iota_types::digests::{ChainIdentifier, get_mainnet_chain_identifier, get_testnet_chain_identifier};
use iota_types::event::EventID;
use poi_rs::{CommitteeResolution, PoiClient, Proof, VerifiedProof};

const GENESIS_CACHE_DIR: &str = "poi";
const MAINNET_GENESIS_URL: &str = "https://dbfiles.mainnet.iota.cafe/genesis.blob";
const TESTNET_GENESIS_URL: &str = "https://dbfiles.testnet.iota.cafe/genesis.blob";
const DEVNET_GENESIS_URL: &str = "https://dbfiles.devnet.iota.cafe/genesis.blob";
const CREATE_EXAMPLES: &str = r#"Examples:
    poi create --network mainnet --transaction TRANSACTION_DIGEST
    poi create --network testnet --object OBJECT_ID --output proof.json
    poi create --grpc-url http://localhost:9000 --event TRANSACTION_DIGEST:EVENT_SEQUENCE

The selected endpoint supplies untrusted proof material; it does not establish verification trust."#;
const VERIFY_EXAMPLES: &str = r#"Examples:
    poi verify --network mainnet proof.json
    poi verify --network testnet --genesis trusted-genesis.blob proof.json
    poi verify --grpc-url http://localhost:9000 --genesis genesis.blob -

Mainnet and testnet download, validate, and cache their genesis blob automatically. Devnet requires --genesis because its genesis digest is not stable. An explicit --genesis path overrides the managed blob.
The genesis blob is the trust anchor. The selected endpoint only supplies committee-walking data."#;

#[derive(Debug, Parser)]
#[command(
    name = "poi",
    version,
    about = "Create and verify IOTA Proof of Inclusion proofs",
    long_about = "Create portable IOTA Proof of Inclusion proofs and verify them against committee history authenticated from a trusted genesis blob.",
    arg_required_else_help = true,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a proof from an IOTA gRPC source.
    Create(CreateArgs),
    /// Verify a proof using genesis-anchored committee history.
    Verify(VerifyArgs),
}

impl Command {
    async fn execute(self) -> Result<()> {
        match self {
            Self::Create(args) => args.execute().await,
            Self::Verify(args) => args.execute().await,
        }
    }
}

#[derive(Debug, Args)]
#[command(
    long_about = "Create a Proof of Inclusion for one transaction and any requested object or event targets that belong to it.",
    after_help = CREATE_EXAMPLES,
    group(
        ArgGroup::new("target")
            .required(true)
            .multiple(true)
            .args(["transaction", "object", "event"])
    )
)]
struct CreateArgs {
    #[command(flatten)]
    endpoint: EndpointArgs,
    /// Transaction digest to prove.
    #[arg(long, value_name = "DIGEST")]
    transaction: Option<TransactionDigest>,
    /// Object ID to prove. Without a transaction or event target, the source resolves its latest version at proof
    /// construction time. May be repeated.
    #[arg(long, value_name = "OBJECT_ID")]
    object: Vec<ObjectId>,
    /// Event identifier formatted as TRANSACTION_DIGEST:EVENT_SEQUENCE. May be repeated.
    #[arg(long, value_name = "EVENT_ID", value_parser = parse_event_id)]
    event: Vec<EventID>,
    /// Output file. Write JSON to stdout when omitted or set to '-'.
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,
}

impl CreateArgs {
    async fn execute(self) -> Result<()> {
        let Self {
            endpoint,
            transaction,
            object,
            event,
            output,
        } = self;
        let client = PoiClient::from_grpc_client(endpoint.client()?);
        let mut builder = client.proof();

        if let Some(transaction) = transaction {
            builder = builder.transaction(transaction);
        }
        let proof = builder
            .objects(object)
            .events(event)
            .build()
            .await
            .context("failed to create proof")?;

        match output.as_deref() {
            Some(path) if path != Path::new("-") => {
                let file = fs::File::create(path)
                    .with_context(|| format!("failed to create proof file '{}'", path.display()))?;
                serde_json::to_writer_pretty(file, &proof)
                    .with_context(|| format!("failed to write proof JSON to '{}'", path.display()))
            }
            _ => serde_json::to_writer_pretty(io::stdout().lock(), &proof)
                .context("failed to write proof JSON to stdout"),
        }
    }
}

#[derive(Debug, Args)]
#[command(
    long_about = "Verify a Proof of Inclusion locally after authenticating the checkpoint committee from a trusted genesis blob.",
    after_help = VERIFY_EXAMPLES
)]
struct VerifyArgs {
    #[command(flatten)]
    endpoint: EndpointArgs,
    /// Proof JSON file, or '-' to read from stdin.
    #[arg(value_name = "PROOF")]
    proof: PathBuf,
    /// Trusted genesis blob. Required with --grpc-url; overrides the managed network blob.
    #[arg(long, value_name = "PATH", required_unless_present = "network")]
    genesis: Option<PathBuf>,
}

impl VerifyArgs {
    async fn execute(self) -> Result<()> {
        let proof: Proof = if self.proof == Path::new("-") {
            serde_json::from_reader(io::stdin().lock()).context("failed to read proof JSON from stdin")?
        } else {
            let file = fs::File::open(&self.proof)
                .with_context(|| format!("failed to open proof file '{}'", self.proof.display()))?;
            serde_json::from_reader(file)
                .with_context(|| format!("failed to read proof JSON from '{}'", self.proof.display()))?
        };
        let genesis = match self.genesis.as_deref() {
            Some(path) => {
                fs::read(path).with_context(|| format!("failed to read genesis blob '{}'", path.display()))?
            }
            None => {
                load_genesis(
                    self.endpoint
                        .network
                        .context("a known network or explicit genesis blob is required for verification")?,
                )
                .await?
            }
        };
        let resolution = CommitteeResolution::from_genesis(genesis.as_slice())
            .map_err(|error| anyhow::anyhow!("failed to load trusted genesis blob: {error}"))?;
        let verified = PoiClient::from_grpc_client(self.endpoint.client()?)
            .verifier(resolution)
            .verify(&proof)
            .await
            .context("proof verification failed")?;
        write_verification_summary(io::stdout().lock(), &verified)
            .context("failed to write verification result to stdout")
    }
}

fn write_verification_summary(mut writer: impl Write, proof: &VerifiedProof<'_>) -> io::Result<()> {
    writeln!(writer, "Proof verified successfully.")?;
    writeln!(writer, "  checkpoint epoch:  {}", proof.checkpoint_epoch())?;
    writeln!(writer, "  checkpoint number: {}", proof.checkpoint_sequence_number())?;
    writeln!(writer, "  timestamp (ms):    {}", proof.checkpoint_timestamp_ms())?;
    writeln!(writer, "  transaction:       {}", proof.transaction_digest())?;
    writeln!(writer, "  targets:")?;

    if let Some(transaction) = proof.transaction_target() {
        writeln!(writer, "    transaction: {transaction}")?;
    }
    for object in proof.objects() {
        let object_ref = object.as_inner().object_ref();
        writeln!(
            writer,
            "    object:      {} @ {} ({})",
            object_ref.object_id, object_ref.version, object_ref.digest
        )?;
    }
    for (event, _) in proof.events() {
        writeln!(writer, "    event:       {}:{}", event.tx_digest, event.event_seq)?;
    }

    Ok(())
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("endpoint")
        .required(true)
        .multiple(false)
        .args(["network", "grpc_url"])
))]
struct EndpointArgs {
    /// Public IOTA network whose default gRPC endpoint should be used.
    #[arg(long, value_enum)]
    network: Option<Network>,
    /// Custom IOTA gRPC endpoint.
    #[arg(long, value_name = "URL")]
    grpc_url: Option<String>,
}

impl EndpointArgs {
    fn client(&self) -> Result<GrpcClient> {
        if let Some(network) = self.network {
            return network.client();
        }
        if let Some(url) = self.grpc_url.as_deref() {
            return GrpcClient::new(url).with_context(|| format!("failed to configure gRPC endpoint '{url}'"));
        }

        bail!("an IOTA network or gRPC URL is required")
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

impl Network {
    const fn name(self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
            Self::Devnet => "devnet",
        }
    }

    const fn genesis_url(self) -> &'static str {
        match self {
            Self::Mainnet => MAINNET_GENESIS_URL,
            Self::Testnet => TESTNET_GENESIS_URL,
            Self::Devnet => DEVNET_GENESIS_URL,
        }
    }

    fn expected_chain_identifier(self) -> Result<ChainIdentifier> {
        match self {
            Self::Mainnet => Ok(get_mainnet_chain_identifier()),
            Self::Testnet => Ok(get_testnet_chain_identifier()),
            Self::Devnet => bail!(
                "managed genesis is unavailable for devnet because its genesis digest is not stable; pass --genesis PATH"
            ),
        }
    }

    fn client(self) -> Result<GrpcClient> {
        match self {
            Self::Mainnet => GrpcClient::new_mainnet().context("failed to configure mainnet gRPC endpoint"),
            Self::Testnet => GrpcClient::new_testnet().context("failed to configure testnet gRPC endpoint"),
            Self::Devnet => GrpcClient::new_devnet().context("failed to configure devnet gRPC endpoint"),
        }
    }
}

async fn load_genesis(network: Network) -> Result<Vec<u8>> {
    network.expected_chain_identifier()?;
    let path = iota_config_dir()
        .context("failed to locate the IOTA configuration directory")?
        .join(GENESIS_CACHE_DIR)
        .join(network.name())
        .join(IOTA_GENESIS_FILENAME);

    let bytes = if path.is_file() {
        let bytes =
            fs::read(&path).with_context(|| format!("failed to read cached genesis blob '{}'", path.display()))?;
        validate_genesis(network, &bytes).with_context(|| {
            format!(
                "cached genesis blob '{}' is not trusted; remove it to download a fresh copy",
                path.display()
            )
        })?;
        bytes
    } else {
        let parent = path
            .parent()
            .context("managed genesis path does not have a parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create genesis cache directory '{}'", parent.display()))?;

        let url = network.genesis_url();
        let bytes = reqwest::get(url)
            .await
            .with_context(|| format!("failed to download {} genesis blob from '{url}'", network.name()))?
            .error_for_status()
            .with_context(|| format!("{} genesis download returned an error", network.name()))?
            .bytes()
            .await
            .with_context(|| format!("failed to read genesis blob from '{url}'"))?
            .to_vec();
        validate_genesis(network, &bytes)
            .with_context(|| format!("downloaded {} genesis blob is not trusted", network.name()))?;
        fs::write(&path, &bytes).with_context(|| format!("failed to cache genesis blob at '{}'", path.display()))?;
        bytes
    };

    Ok(bytes)
}

/// Validates that the genesis blob is valid BCS and matches the expected chain
/// identifier for the network.
fn validate_genesis(network: Network, bytes: &[u8]) -> Result<()> {
    let genesis: Genesis =
        bcs::from_bytes(bytes).with_context(|| format!("{} genesis blob is not valid BCS", network.name()))?;
    let actual = ChainIdentifier::from(*genesis.checkpoint().digest());
    let expected = network.expected_chain_identifier()?;
    ensure!(
        actual == expected,
        "{} genesis digest mismatch: expected {}, found {}",
        network.name(),
        expected.digest(),
        actual.digest()
    );
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    Cli::parse().command.execute().await
}

fn parse_event_id(value: &str) -> Result<EventID, String> {
    EventID::try_from(value.to_owned())
        .map_err(|error| format!("invalid event ID '{value}'; expected TRANSACTION_DIGEST:EVENT_SEQUENCE: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_genesis_uses_the_pinned_network_identifiers() {
        assert_eq!(
            Network::Mainnet
                .expected_chain_identifier()
                .expect("mainnet must have a pinned identifier"),
            get_mainnet_chain_identifier()
        );
        assert_eq!(
            Network::Testnet
                .expected_chain_identifier()
                .expect("testnet must have a pinned identifier"),
            get_testnet_chain_identifier()
        );
    }

    #[test]
    fn managed_genesis_is_unavailable_for_devnet() {
        let error = Network::Devnet
            .expected_chain_identifier()
            .expect_err("devnet must require an explicit genesis blob");

        assert!(error.to_string().contains("pass --genesis PATH"));
    }

    #[test]
    fn managed_genesis_rejects_invalid_bcs() {
        let error =
            validate_genesis(Network::Mainnet, b"not a genesis blob").expect_err("invalid BCS must not be trusted");

        assert!(error.to_string().contains("mainnet genesis blob is not valid BCS"));
    }
}
