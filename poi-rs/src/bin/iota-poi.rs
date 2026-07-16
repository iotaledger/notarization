// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![deny(clippy::print_stderr, clippy::print_stdout)]

use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
use iota_config::{IOTA_GENESIS_FILENAME, genesis::Genesis, iota_config_dir};
use iota_grpc_client::Client as GrpcClient;
use iota_sdk_types::ObjectId;
use iota_types::{digests::TransactionDigest, event::EventID};
use poi_rs::{CommitteeResolver, Proof, ProofBuilder, ProofVerifier};
use tempfile::NamedTempFile;

const STDIO_PATH: &str = "-";
const GENESIS_CACHE_DIR: &str = "iota-poi";
const GENESIS_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_GENESIS_BLOB_BYTES: usize = 64 * 1024 * 1024;
const MAINNET_GENESIS_URL: &str = "https://dbfiles.mainnet.iota.cafe/genesis.blob";
const TESTNET_GENESIS_URL: &str = "https://dbfiles.testnet.iota.cafe/genesis.blob";
const DEVNET_GENESIS_URL: &str = "https://dbfiles.devnet.iota.cafe/genesis.blob";
const CREATE_EXAMPLES: &str = r#"Examples:
  iota-poi create --network mainnet --transaction TRANSACTION_DIGEST
  iota-poi create --network testnet --object OBJECT_ID --output proof.json
  iota-poi create --grpc-url http://localhost:9000 --event TRANSACTION_DIGEST:EVENT_SEQUENCE

The selected endpoint supplies untrusted proof material; it does not establish verification trust."#;
const VERIFY_EXAMPLES: &str = r#"Examples:
  iota-poi verify --network mainnet proof.json
  iota-poi verify --network testnet --genesis trusted-genesis.blob proof.json
  iota-poi verify --grpc-url http://localhost:9000 --genesis genesis.blob -

Known networks download and cache their genesis blob automatically. An explicit --genesis path overrides the managed blob.
The genesis blob is the trust anchor. The selected endpoint only supplies committee-walking data."#;

#[derive(Debug, Parser)]
#[command(
    name = "iota-poi",
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
        let json = proof.to_json_vec().context("failed to encode proof as JSON")?;

        write_output(output.as_deref(), &json)
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
        let proof_bytes = read_input(&self.proof)?;
        let proof = Proof::from_json_slice(&proof_bytes).context("failed to decode proof JSON")?;
        proof.validate().context("proof format is not supported")?;

        let genesis = match self.genesis.as_deref() {
            Some(path) => {
                Genesis::load(path).with_context(|| format!("failed to load genesis blob '{}'", path.display()))?
            }
            None => {
                download_or_load_genesis(
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
        write_stdout(b"valid")
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

async fn download_or_load_genesis(network: Network) -> Result<Genesis> {
    let path = iota_config_dir()
        .context("failed to locate the IOTA configuration directory")?
        .join(GENESIS_CACHE_DIR)
        .join(network.name())
        .join(IOTA_GENESIS_FILENAME);

    if path.is_file() {
        if let Ok(genesis) = Genesis::load(&path) {
            return Ok(genesis);
        }
    }

    let parent = path
        .parent()
        .context("managed genesis path does not have a parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create genesis cache directory '{}'", parent.display()))?;

    let url = network.genesis_url();
    let response = reqwest::Client::builder()
        .timeout(GENESIS_DOWNLOAD_TIMEOUT)
        .user_agent(concat!("iota-poi/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to configure the genesis download client")?
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to download {} genesis blob from '{url}'", network.name()))?
        .error_for_status()
        .with_context(|| format!("genesis server rejected the request to '{url}'"))?;

    if response
        .content_length()
        .is_some_and(|length| length > MAX_GENESIS_BLOB_BYTES as u64)
    {
        bail!("genesis blob from '{url}' exceeds the {MAX_GENESIS_BLOB_BYTES}-byte size limit");
    }

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("failed to read genesis blob from '{url}'"))?;
    if bytes.len() > MAX_GENESIS_BLOB_BYTES {
        bail!("genesis blob from '{url}' exceeds the {MAX_GENESIS_BLOB_BYTES}-byte size limit");
    }

    let mut temporary = NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create a temporary genesis file in '{}'", parent.display()))?;
    temporary
        .write_all(&bytes)
        .with_context(|| format!("failed to write downloaded {} genesis blob", network.name()))?;
    temporary
        .as_file()
        .sync_all()
        .with_context(|| format!("failed to flush downloaded {} genesis blob", network.name()))?;
    let genesis = Genesis::load(temporary.path())
        .with_context(|| format!("downloaded {} genesis blob from '{url}' is invalid", network.name()))?;
    temporary
        .persist(&path)
        .map_err(|error| error.error)
        .with_context(|| format!("failed to cache genesis blob at '{}'", path.display()))?;

    Ok(genesis)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if is_broken_pipe(&error) => ExitCode::SUCCESS,
        Err(error) => {
            report_error(&error);
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    Cli::parse().command.execute().await
}

fn report_error(error: &anyhow::Error) {
    let _ = writeln!(io::stderr().lock(), "error: {error:#}");
}

fn is_broken_pipe(error: &anyhow::Error) -> bool {
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(error.as_ref());

    while let Some(error) = cause {
        if error
            .downcast_ref::<io::Error>()
            .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
        {
            return true;
        }

        cause = error.source();
    }

    false
}

fn read_input(path: &Path) -> Result<Vec<u8>> {
    if is_stdio(path) {
        let mut bytes = Vec::new();
        io::stdin()
            .lock()
            .read_to_end(&mut bytes)
            .context("failed to read proof JSON from stdin")?;
        return Ok(bytes);
    }

    fs::read(path).with_context(|| format!("failed to read proof JSON from '{}'", path.display()))
}

fn write_output(path: Option<&Path>, bytes: &[u8]) -> Result<()> {
    match path {
        None => write_stdout(bytes),
        Some(path) if is_stdio(path) => write_stdout(bytes),
        Some(path) => write_file_atomically(path, bytes),
    }
}

fn write_stdout(bytes: &[u8]) -> Result<()> {
    write_json(io::stdout().lock(), bytes).context("failed to write to stdout")
}

fn write_file_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create a temporary output file in '{}'", parent.display()))?;

    write_json(&mut temporary, bytes)
        .with_context(|| format!("failed to write proof JSON for '{}'", path.display()))?;
    temporary
        .as_file()
        .sync_all()
        .with_context(|| format!("failed to flush proof JSON for '{}'", path.display()))?;
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| error.error)
        .with_context(|| format!("failed to atomically replace output file '{}'", path.display()))
}

fn write_json(mut writer: impl Write, bytes: &[u8]) -> io::Result<()> {
    writer.write_all(bytes)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn is_stdio(path: &Path) -> bool {
    path == Path::new(STDIO_PATH)
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
        let error = Cli::try_parse_from(["iota-poi", "create", "--network", "mainnet"])
            .expect_err("create without a target must fail");

        assert!(error.to_string().contains("--transaction <DIGEST>"));
    }

    #[test]
    fn create_accepts_mixed_targets() {
        let event = format!("{DIGEST}:0");
        let cli = Cli::try_parse_from([
            "iota-poi",
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
            "iota-poi",
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
        let cli = Cli::try_parse_from(["iota-poi", "verify", "--network", "mainnet", "proof.json"])
            .expect("known network must not require an explicit genesis blob");

        let Command::Verify(args) = cli.command else {
            panic!("verify command must parse");
        };
        assert!(args.genesis.is_none());
    }

    #[test]
    fn custom_endpoint_verification_requires_genesis() {
        let error = Cli::try_parse_from([
            "iota-poi",
            "verify",
            "--grpc-url",
            "http://localhost:9000",
            "proof.json",
        ])
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
        let error = Cli::try_parse_from(["iota-poi", "create", "--network", "mainnet", "--event", "not-an-event"])
            .expect_err("invalid event ID must fail");

        assert!(error.to_string().contains("TRANSACTION_DIGEST:EVENT_SEQUENCE"));
    }

    #[test]
    fn invalid_object_id_reports_the_invalid_value() {
        let error = Cli::try_parse_from([
            "iota-poi",
            "create",
            "--network",
            "mainnet",
            "--object",
            "not-an-object",
        ])
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

    #[test]
    fn file_output_is_newline_terminated_and_replaced_atomically() {
        let directory = tempfile::tempdir().expect("temporary directory must be created");
        let output = directory.path().join("proof.json");

        write_output(Some(&output), br#"{"version":1}"#).expect("initial proof must be written");
        assert_eq!(fs::read(&output).unwrap(), b"{\"version\":1}\n");

        write_output(Some(&output), br#"{"version":2}"#).expect("proof must be replaced");
        assert_eq!(fs::read(&output).unwrap(), b"{\"version\":2}\n");
    }

    #[test]
    fn broken_pipe_is_treated_as_a_successful_pipeline_shutdown() {
        let error = anyhow::Error::new(io::Error::new(io::ErrorKind::BrokenPipe, "reader closed"))
            .context("failed to write proof");

        assert!(is_broken_pipe(&error));
    }
}
