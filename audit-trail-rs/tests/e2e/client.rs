// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;
use std::sync::Arc;
use std::sync::OnceLock;

use audit_trails::AuditTrailClient;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::crypto::PublicKey;
use iota_interaction::{IOTA_LOCAL_NETWORK_URL, IotaClientBuilder, KeytoolSigner};
use iota_interaction_rust::IotaClientAdapter;
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::network_name::NetworkName;
use product_common::test_utils::{
    TEST_GAS_BUDGET, get_active_address, get_balance, init_product_package, request_funds,
};
use tokio::sync::{Mutex, MutexGuard, OnceCell};

static PACKAGE_ID: OnceCell<ObjectID> = OnceCell::const_new();
static E2E_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Script file for publishing the package.
pub const PUBLISH_SCRIPT_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../audit-trail-move/scripts/publish_package.sh"
);

pub async fn get_funded_test_client() -> anyhow::Result<TestClient> {
    TestClient::new().await
}

pub async fn e2e_test_guard() -> MutexGuard<'static, ()> {
    E2E_MUTEX.get_or_init(|| Mutex::new(())).lock().await
}

#[derive(Clone)]
pub struct TestClient {
    client: Arc<AuditTrailClient<KeytoolSigner>>,
}

impl Deref for TestClient {
    type Target = AuditTrailClient<KeytoolSigner>;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl TestClient {
    pub async fn new() -> anyhow::Result<Self> {
        let active_address = get_active_address().await?;
        Self::new_from_address(active_address).await
    }

    pub async fn new_from_address(address: IotaAddress) -> anyhow::Result<Self> {
        let api_endpoint = std::env::var("API_ENDPOINT").unwrap_or_else(|_| IOTA_LOCAL_NETWORK_URL.to_string());
        let client = IotaClientBuilder::default().build(&api_endpoint).await?;
        let package_id = PACKAGE_ID
            .get_or_try_init(|| init_product_package(&client, None, Some(PUBLISH_SCRIPT_FILE)))
            .await
            .copied()?;

        let balance = get_balance(address).await?;
        if balance < TEST_GAS_BUDGET {
            request_funds(&address).await?;
        }

        let signer = KeytoolSigner::builder().build()?;
        let client = AuditTrailClient::from_iota_client(client.clone(), Some(package_id)).await?;
        let client = client.with_signer(signer).await?;

        Ok(TestClient {
            client: Arc::new(client),
        })
    }
}

impl CoreClientReadOnly for TestClient {
    fn package_id(&self) -> ObjectID {
        self.client.package_id()
    }

    fn network_name(&self) -> &NetworkName {
        self.client.network_name()
    }

    fn client_adapter(&self) -> &IotaClientAdapter {
        self.client.client_adapter()
    }
}

impl CoreClient<KeytoolSigner> for TestClient {
    fn signer(&self) -> &KeytoolSigner {
        self.client.signer()
    }

    fn sender_address(&self) -> IotaAddress {
        self.client.sender_address()
    }

    fn sender_public_key(&self) -> &PublicKey {
        self.client.sender_public_key()
    }
}
