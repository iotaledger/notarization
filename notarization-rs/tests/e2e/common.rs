// // Copyright 2020-2025 IOTA Stiftung
// // SPDX-License-Identifier: Apache-2.0
// #![allow(dead_code)]

// use lazy_static::lazy_static;
// use anyhow::anyhow;
// use anyhow::Context;

use identity_jose::jwk::Jwk;
use identity_jose::jws::JwsAlgorithm;
use identity_storage::JwkMemStore;
use identity_storage::KeyId;
use identity_storage::KeyIdMemstore;
use identity_storage::KeyType;
use identity_storage::Storage;
use identity_storage::StorageSigner;
use identity_iota_core::rebased::transaction::Transaction;
use identity_iota_interaction::{IotaClient, IotaClientBuilder, IOTA_LOCAL_NETWORK_URL};
use identity_iota_interaction::types::crypto::SignatureScheme;
use secret_storage::Signer;
use serde::Deserialize;
use serde_json::Value;
use std::io::Write;
use std::str::FromStr;
use std::sync::Arc;
use identity_iota_interaction::IotaKeySignature;
use identity_iota_interaction::keytool_signer::KeytoolSigner;
use identity_iota_interaction::types::base_types::{IotaAddress, ObjectID};
use move_core_types::language_storage::StructTag;
use tokio::process::Command;
use tokio::sync::OnceCell;

// use notarization::client_tools::request_funds;

// pub type MemStorage = Storage<JwkMemStore, KeyIdMemstore>;
// pub type MemSigner<'s> = StorageSigner<'s, JwkMemStore, KeyIdMemstore>;

// static PACKAGE_ID: OnceCell<ObjectID> = OnceCell::const_new();
// const SCRIPT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/");
// const CACHED_PRODUCT_PKG_ID: &str = "/tmp/iota_product_pkg_id.txt";

// pub const TEST_GAS_BUDGET: u64 = 50_000_000;

// lazy_static! {
//   pub static ref TEST_COIN_TYPE: StructTag = "0x2::coin::Coin<bool>".parse().unwrap();
// }

// pub struct InitialAccountData<S: Signer<IotaKeySignature>> {
//   pub signer: S,
//   pub iota_client: IotaClient,
// }

// async fn init_package(iota_client: &IotaClient) -> anyhow::Result<ObjectID> {
//   let network_id = iota_client.read_api().get_chain_identifier().await?;
//   let address = get_active_address().await?;

//   if let Ok(id) = std::env::var("IOTA_PRODUCT_PKG_ID").or(get_cached_product_pkg_id(&network_id).await) {
//     std::env::set_var("IOTA_PRODUCT_PKG_ID", id.clone());
//     id.parse().context("failed to parse object id from str")
//   } else {
//     publish_package(address).await
//   }
// }

// async fn get_cached_product_pkg_id(network_id: &str) -> anyhow::Result<String> {
//   let cache = tokio::fs::read_to_string(CACHED_PRODUCT_PKG_ID).await?;
//   let (cached_id, cached_network_id) = cache.split_once(';').ok_or(anyhow!("Invalid or empty cached data"))?;

//   if cached_network_id == network_id {
//     Ok(cached_id.to_owned())
//   } else {
//     Err(anyhow!("A network change has invalidated the cached data"))
//   }
// }

// /// Returns the default address of the IOTA CLI installed on the local system (`iota client active-address`)
// async fn get_active_address() -> anyhow::Result<IotaAddress> {
//   Command::new("iota")
//     .arg("client")
//     .arg("active-address")
//     .arg("--json")
//     .output()
//     .await
//     .context("Failed to execute command")
//     .and_then(|output| Ok(serde_json::from_slice::<IotaAddress>(&output.stdout)?))
// }

// async fn publish_package(active_address: IotaAddress) -> anyhow::Result<ObjectID> {
//   let output = Command::new("sh")
//     .current_dir(SCRIPT_DIR)
//     .arg("publish_identity_package.sh")
//     .output()
//     .await?;
//   let stdout = std::str::from_utf8(&output.stdout).unwrap();

//   if !output.status.success() {
//     let stderr = std::str::from_utf8(&output.stderr).unwrap();
//     anyhow::bail!("Failed to publish move package: \n\n{stdout}\n\n{stderr}");
//   }

//   let package_id: ObjectID = {
//     let stdout_trimmed = stdout.trim();
//     ObjectID::from_str(stdout_trimmed).with_context(|| {
//       let stderr = std::str::from_utf8(&output.stderr).unwrap();
//       format!("failed to find IDENTITY_IOTA_PKG_ID in response from: '{stdout_trimmed}'; {stderr}")
//     })?
//   };

//   // Persist package ID in order to avoid publishing the package for every test.
//   let package_id_str = package_id.to_string();
//   std::env::set_var("IDENTITY_IOTA_PKG_ID", package_id_str.as_str());
//   let mut file = std::fs::File::create(CACHED_PRODUCT_PKG_ID)?;
//   write!(&mut file, "{};{}", package_id_str, active_address)?;

//   Ok(package_id)
// }

// pub async fn get_key_data() -> Result<(Storage<JwkMemStore, KeyIdMemstore>, KeyId, Jwk, Vec<u8>), anyhow::Error> {
//   let storage = Storage::<JwkMemStore, KeyIdMemstore>::new(JwkMemStore::new(), KeyIdMemstore::new());
//   let generate = storage
//     .key_storage()
//     .generate(KeyType::new("Ed25519"), JwsAlgorithm::EdDSA)
//     .await?;
//   let public_key_jwk = generate.jwk.to_public().expect("public components should be derivable");
//   let public_key_bytes = get_public_key_bytes(&public_key_jwk)?;
//   // let sender_address = convert_to_address(&public_key_bytes)?;

//   Ok((storage, generate.key_id, public_key_jwk, public_key_bytes))
// }

// fn get_public_key_bytes(sender_public_jwk: &Jwk) -> Result<Vec<u8>, anyhow::Error> {
//   let public_key_base_64 = &sender_public_jwk
//     .try_okp_params()
//     .map_err(|err| anyhow!("key not of type `Okp`; {err}"))?
//     .x;

//   identity_jose::jwu::decode_b64(public_key_base_64).map_err(|err| anyhow!("could not decode base64 public key; {err}"))
// }

// #[derive(Deserialize)]
// #[serde(rename_all = "camelCase")]
// struct GasObjectHelper {
//   nanos_balance: u64,
// }

// async fn get_balance(address: IotaAddress) -> anyhow::Result<u64> {
//   let output = Command::new("iota")
//     .arg("client")
//     .arg("gas")
//     .arg("--json")
//     .arg(address.to_string())
//     .output()
//     .await?;

//   if !output.status.success() {
//     let error_msg = String::from_utf8(output.stderr)?;
//     anyhow::bail!("failed to get balance: {error_msg}");
//   }

//   let balance = serde_json::from_slice::<Vec<GasObjectHelper>>(&output.stdout)?
//     .into_iter()
//     .map(|gas_info| gas_info.nanos_balance)
//     .sum();

//   Ok(balance)
// }

// #[derive(Clone)]
// pub struct TestClient {
//   client: Arc<IotaClient>,
//   signer: Arc<KeytoolSigner>,
//   product_pkg_id: ObjectID,
//   storage: Arc<MemStorage>,
// }

// impl TestClient {
//   pub async fn new() -> anyhow::Result<Self> {
//     let active_address = get_active_address().await?;
//     Self::new_from_address(active_address).await
//   }

//   pub async fn new_from_address(address: IotaAddress) -> anyhow::Result<Self> {
//     let api_endpoint = std::env::var("API_ENDPOINT").unwrap_or_else(|_| IOTA_LOCAL_NETWORK_URL.to_string());
//     println!("api_endpoint: {}", api_endpoint);
//     let iota_client = IotaClientBuilder::default().build(&api_endpoint).await?;
//     let product_pkg_id = PACKAGE_ID.get_or_try_init(|| init_package(&iota_client)).await.copied()?;

//     let balance = get_balance(address).await?;
//     if balance < TEST_GAS_BUDGET {
//       request_funds(&address).await?;
//     }

//     let storage = Arc::new(Storage::new(JwkMemStore::new(), KeyIdMemstore::new()));
//     // Create a signer using the currently active address of the iota CLI (default)
//     let signer = KeytoolSigner::builder().build().await?;

//     Ok(TestClient {
//       client: Arc::new(iota_client),
//       signer: Arc::new(signer),
//       product_pkg_id,
//       storage,
//     })
//   }

//   pub async fn get_funded_client_account(&self) -> anyhow::Result<InitialAccountData<MemSigner>> {
//     let storage = Arc::new(Storage::new(JwkMemStore::new(), KeyIdMemstore::new()));
//     let generate = storage
//       .key_storage()
//       .generate(KeyType::new("Ed25519"), JwsAlgorithm::EdDSA)
//       .await?;
//     let public_key_jwk = generate.jwk.to_public().expect("public components should be derivable");
//     let signer = StorageSigner::new(&self.storage, generate.key_id, public_key_jwk);
//     let public_key = <MemSigner as Signer<IotaKeySignature>>::public_key(&signer).await?;

//     request_funds(&IotaAddress::from(&public_key)).await?;

//     Ok(InitialAccountData{signer, iota_client: (*self.client).clone() })
//   }


//   pub async fn new_with_key_type(key_type: SignatureScheme) -> anyhow::Result<Self> {
//     let address = make_address(key_type).await?;
//     Self::new_from_address(address).await
//   }

//   pub fn package_id(&self) -> ObjectID {
//     self.product_pkg_id
//   }

//   pub fn signer(&self) -> &KeytoolSigner {
//     &self.signer
//   }
// }

// pub async fn make_address(key_type: SignatureScheme) -> anyhow::Result<IotaAddress> {
//   if !matches!(
//     key_type,
//     SignatureScheme::ED25519 | SignatureScheme::Secp256k1 | SignatureScheme::Secp256r1
//   ) {
//     anyhow::bail!("key type {key_type} is not supported");
//   }

//   let output = Command::new("iota")
//     .arg("client")
//     .arg("new-address")
//     .arg("--key-scheme")
//     .arg(key_type.to_string())
//     .arg("--json")
//     .output()
//     .await?;
//   let new_address = {
//     let stdout = std::str::from_utf8(&output.stdout).unwrap();
//     let start_of_json = stdout.find('{').ok_or_else(|| {
//       let stderr = std::str::from_utf8(&output.stderr).unwrap();
//       anyhow!("No json in output: '{stdout}'; {stderr}",)
//     })?;
//     let json_result = serde_json::from_str::<Value>(stdout[start_of_json..].trim())?;
//     let address_str = json_result
//       .get("address")
//       .context("no address in JSON output")?
//       .as_str()
//       .context("address is not a JSON string")?;

//     address_str.parse()?
//   };

//   request_funds(&new_address).await?;

//   Ok(new_address)
// }

// pub async fn get_funded_test_client() -> anyhow::Result<TestClient> {
//   TestClient::new().await
// }
