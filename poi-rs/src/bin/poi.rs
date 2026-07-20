// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
use iota_config::{IOTA_GENESIS_FILENAME, genesis::Genesis, iota_config_dir};
use iota_grpc_client::Client as GrpcClient;
use iota_sdk_types::ObjectId;
use iota_types::{digests::TransactionDigest, event::EventID};
use poi_rs::{CommitteeResolver, Proof, ProofBuilder, ProofVerifier};

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

Known networks download and cache their genesis blob automatically. An explicit --genesis path overrides the managed blob.
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
    #[arg(long, value_name = "DIGEST", value_parser = parse_transaction_digest, group = "target")]
    transaction: Option<TransactionDigest>,
    /// Object ID to prove. The source resolves its latest version unless a transaction or event scopes the proof. May be repeated.
    #[arg(long, value_name = "OBJECT_ID", value_parser = parse_object_id, group = "target")]
    object: Vec<ObjectId>,
    /// Event identifier formatted as TRANSACTION_DIGEST:EVENT_SEQUENCE. May be repeated.
    #[arg(long, value_name = "EVENT_ID", value_parser = parse_event_id, group = "target")]
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
        let mut builder = ProofBuilder::from_grpc_client(endpoint.client()?);

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
        proof.validate().context("proof format is not supported")?;

        let genesis = match self.genesis.as_deref() {
            Some(path) => {
                Genesis::load(path).with_context(|| format!("failed to load genesis blob '{}'", path.display()))?
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
        let trusted_committee = genesis
            .committee()
            .context("failed to read committee from genesis blob")?;
        let resolver = CommitteeResolver::anchor(self.endpoint.client()?, trusted_committee);
        let committee = resolver
            .resolve(proof.checkpoint_summary.epoch())
            .await
            .context("failed to authenticate the proof checkpoint committee")?;

        ProofVerifier::new(&committee)
            .verify(&proof)
            .context("proof verification failed")?;
        writeln!(io::stdout().lock(), "valid").context("failed to write verification result to stdout")
    }
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

    fn client(self) -> Result<GrpcClient> {
        match self {
            Self::Mainnet => GrpcClient::new_mainnet().context("failed to configure mainnet gRPC endpoint"),
            Self::Testnet => GrpcClient::new_testnet().context("failed to configure testnet gRPC endpoint"),
            Self::Devnet => GrpcClient::new_devnet().context("failed to configure devnet gRPC endpoint"),
        }
    }
}

async fn load_genesis(network: Network) -> Result<Genesis> {
    let path = iota_config_dir()
        .context("failed to locate the IOTA configuration directory")?
        .join(GENESIS_CACHE_DIR)
        .join(network.name())
        .join(IOTA_GENESIS_FILENAME);

    if !path.is_file() {
        let parent = path
            .parent()
            .context("managed genesis path does not have a parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create genesis cache directory '{}'", parent.display()))?;

        let url = network.genesis_url();
        let bytes = reqwest::get(url)
            .await
            .with_context(|| format!("failed to download {} genesis blob from '{url}'", network.name()))?
            .bytes()
            .await
            .with_context(|| format!("failed to read genesis blob from '{url}'"))?;
        fs::write(&path, bytes).with_context(|| format!("failed to cache genesis blob at '{}'", path.display()))?;
    }

    Genesis::load(&path).with_context(|| format!("failed to load genesis blob '{}'", path.display()))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    Cli::parse().command.execute().await
}

fn parse_transaction_digest(value: &str) -> Result<TransactionDigest, String> {
    value
        .parse()
        .map_err(|error| format!("invalid transaction digest '{value}': {error}"))
}

fn parse_object_id(value: &str) -> Result<ObjectId, String> {
    value
        .parse::<ObjectId>()
        .map_err(|error| format!("invalid object ID '{value}': {error}"))
}

fn parse_event_id(value: &str) -> Result<EventID, String> {
    let mut parts = value.split(':');
    let (Some(transaction), Some(sequence), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(format!(
            "invalid event ID '{value}'; expected TRANSACTION_DIGEST:EVENT_SEQUENCE"
        ));
    };
    let tx_digest = transaction
        .parse::<TransactionDigest>()
        .map_err(|error| format!("invalid transaction digest in event ID '{value}': {error}"))?;
    let event_seq = sequence
        .parse::<u64>()
        .map_err(|error| format!("invalid event sequence in '{value}': {error}"))?;

    Ok(EventID { tx_digest, event_seq })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    const DIGEST: &str = "11111111111111111111111111111111";
    const OBJECT_ID: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn create_requires_a_target() {
        let error = Cli::try_parse_from(["poi", "create", "--network", "mainnet"])
            .expect_err("create without a target must fail");

        assert!(error.to_string().contains("--transaction <DIGEST>"));
    }

    #[test]
    fn create_accepts_mixed_targets() {
        let event = format!("{DIGEST}:0");
        let cli = Cli::try_parse_from([
            "poi",
            "create",
            "--network",
            "testnet",
            "--transaction",
            DIGEST,
            "--object",
            OBJECT_ID,
            "--event",
            &event,
        ])
        .expect("mixed targets must parse");

        let Command::Create(args) = cli.command else {
            panic!("create command must parse");
        };
        assert!(args.transaction.is_some());
        assert_eq!(args.object.len(), 1);
        assert_eq!(args.event.len(), 1);
    }

    #[test]
    fn endpoint_selection_is_exclusive() {
        let error = Cli::try_parse_from([
            "poi",
            "create",
            "--network",
            "mainnet",
            "--grpc-url",
            "http://localhost:9000",
            "--transaction",
            DIGEST,
        ])
        .expect_err("multiple endpoints must fail");

        assert!(error.to_string().contains("cannot be used with"));
    }

    #[test]
    fn known_network_verification_manages_genesis_automatically() {
        let cli = Cli::try_parse_from(["poi", "verify", "--network", "mainnet", "proof.json"])
            .expect("known network must not require an explicit genesis blob");

        let Command::Verify(args) = cli.command else {
            panic!("verify command must parse");
        };
        assert!(args.genesis.is_none());
    }

    #[test]
    fn custom_endpoint_verification_requires_genesis() {
        let error = Cli::try_parse_from(["poi", "verify", "--grpc-url", "http://localhost:9000", "proof.json"])
            .expect_err("custom endpoint must require an explicit genesis blob");

        assert!(error.to_string().contains("--genesis <PATH>"));
    }

    #[test]
    fn known_network_genesis_urls_match_the_iota_light_client() {
        assert_eq!(Network::Mainnet.genesis_url(), MAINNET_GENESIS_URL);
        assert_eq!(Network::Testnet.genesis_url(), TESTNET_GENESIS_URL);
        assert_eq!(Network::Devnet.genesis_url(), DEVNET_GENESIS_URL);
    }

    #[test]
    fn invalid_event_id_reports_the_required_format() {
        let error = Cli::try_parse_from(["poi", "create", "--network", "mainnet", "--event", "not-an-event"])
            .expect_err("invalid event ID must fail");

        assert!(error.to_string().contains("TRANSACTION_DIGEST:EVENT_SEQUENCE"));
    }

    #[test]
    fn invalid_object_id_reports_the_invalid_value() {
        let error = Cli::try_parse_from(["poi", "create", "--network", "mainnet", "--object", "not-an-object"])
            .expect_err("invalid object ID must fail");

        assert!(error.to_string().contains("invalid object ID 'not-an-object'"));
    }

    #[test]
    fn command_help_explains_the_trust_boundary() {
        let mut command = Cli::command();
        let create = command
            .find_subcommand_mut("create")
            .expect("create subcommand must exist")
            .render_long_help()
            .to_string();
        let verify = command
            .find_subcommand_mut("verify")
            .expect("verify subcommand must exist")
            .render_long_help()
            .to_string();

        assert!(create.contains("does not establish verification trust"));
        assert!(verify.contains("genesis blob is the trust anchor"));
    }
}
