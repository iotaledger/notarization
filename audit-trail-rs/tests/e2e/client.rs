// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;
use std::sync::Arc;

use audit_trails::AuditTrailClient;
use audit_trails::core::types::Capability;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::crypto::PublicKey;
use iota_interaction::{IOTA_LOCAL_NETWORK_URL, IotaClientBuilder};
use iota_interaction_rust::IotaClientAdapter;
use iota_sdk::types::base_types::ObjectRef;
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::network_name::NetworkName;
use product_common::test_utils::{InMemSigner, init_product_package, request_funds};
use tokio::sync::OnceCell;

static PACKAGE_ID: OnceCell<ObjectID> = OnceCell::const_new();

/// Script file for publishing the package.
pub const PUBLISH_SCRIPT_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../audit-trail-move/scripts/publish_package.sh"
);

pub async fn get_funded_test_client() -> anyhow::Result<TestClient> {
    TestClient::new().await
}

#[derive(Clone)]
pub struct TestClient {
    client: Arc<AuditTrailClient<InMemSigner>>,
}

impl Deref for TestClient {
    type Target = AuditTrailClient<InMemSigner>;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl TestClient {
    pub async fn new() -> anyhow::Result<Self> {
        let api_endpoint = std::env::var("API_ENDPOINT").unwrap_or_else(|_| IOTA_LOCAL_NETWORK_URL.to_string());
        let iota_client = IotaClientBuilder::default().build(&api_endpoint).await?;
        let package_id = PACKAGE_ID
            .get_or_try_init(|| init_product_package(&iota_client, None, Some(PUBLISH_SCRIPT_FILE)))
            .await
            .copied()?;

        // Use a dedicated ephemeral signer per test to avoid object-lock contention.
        let signer = InMemSigner::new();
        let signer_address = signer.get_address().await?;
        request_funds(&signer_address).await?;

        let client = AuditTrailClient::from_iota_client(iota_client.clone(), Some(package_id)).await?;
        let client = client.with_signer(signer).await?;

        Ok(TestClient {
            client: Arc::new(client),
        })
    }

    pub(crate) async fn get_cap(&self, owner: IotaAddress, trail_id: ObjectID) -> anyhow::Result<ObjectRef> {
        let cap: Capability = self
            .client
            .find_object_for_address(owner, |cap: &Capability| cap.target_key == trail_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to find accredit cap for owner {owner} and trail {trail_id}: {e}"))?
            .ok_or_else(|| anyhow::anyhow!("No accredit capability found for owner {owner} and trail {trail_id}"))?;

        let object_id = *cap.id.object_id();

        Ok(self
            .client
            .get_object_ref_by_id(object_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get object ref for accredit cap: {e}"))?
            .map(|owned_ref| owned_ref.reference.to_object_ref())
            .unwrap())
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

impl CoreClient<InMemSigner> for TestClient {
    fn signer(&self) -> &InMemSigner {
        self.client.signer()
    }

    fn sender_address(&self) -> IotaAddress {
        self.client.sender_address()
    }

    fn sender_public_key(&self) -> &PublicKey {
        self.client.sender_public_key()
    }
}
