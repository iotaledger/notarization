// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use audit_trails::core::types::{
    Capability, CapabilityIssueOptions, CapabilityIssued, Data, InitialRecord, Permission, PermissionSet, RecordTags,
    RoleCreated,
};
use audit_trails::{AuditTrailClient, PackageOverrides};
use iota_interaction::types::base_types::{IotaAddress, ObjectID, ObjectRef};
use iota_interaction::types::crypto::PublicKey;
use iota_interaction::{IOTA_LOCAL_NETWORK_URL, IotaClient, IotaClientBuilder};
use iota_interaction_rust::IotaClientAdapter;
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::network_name::NetworkName;
use product_common::test_utils::{InMemSigner, request_funds};
use tokio::fs;
use tokio::process::Command;
use tokio::sync::OnceCell;

static PACKAGE_IDS: OnceCell<PublishedPackageIds> = OnceCell::const_new();

/// Script file for publishing the package.
pub const PUBLISH_SCRIPT_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../audit-trail-move/scripts/publish_package.sh"
);

const CACHED_PKG_FILE: &str = "/tmp/audit_trail_pkg_ids.txt";

#[derive(Clone, Copy)]
struct PublishedPackageIds {
    audit_trail_package_id: ObjectID,
    tf_components_package_id: Option<ObjectID>,
}

pub async fn get_funded_test_client() -> anyhow::Result<TestClient> {
    TestClient::new().await
}

async fn load_cached_package_ids(chain_id: &str) -> anyhow::Result<PublishedPackageIds> {
    let cache = fs::read_to_string(CACHED_PKG_FILE).await?;
    let mut parts = cache.trim().split(';');
    let audit_trail_package_id = parts
        .next()
        .ok_or_else(|| anyhow!("missing audit_trail package ID in cache"))?;
    let tf_components_package_id = parts.next().unwrap_or_default();
    let cached_chain_id = parts.next().ok_or_else(|| anyhow!("missing chain ID in cache"))?;

    if cached_chain_id != chain_id {
        anyhow::bail!("cached package IDs belong to a different chain");
    }

    Ok(PublishedPackageIds {
        audit_trail_package_id: ObjectID::from_str(audit_trail_package_id)
            .context("failed to parse cached audit_trail package ID")?,
        tf_components_package_id: if tf_components_package_id.is_empty() {
            None
        } else {
            Some(
                ObjectID::from_str(tf_components_package_id)
                    .context("failed to parse cached TfComponents package ID")?,
            )
        },
    })
}

async fn publish_package_ids(iota_client: &IotaClient) -> anyhow::Result<PublishedPackageIds> {
    let chain_id = iota_client
        .read_api()
        .get_chain_identifier()
        .await
        .map_err(|e| anyhow!(e.to_string()))?;

    if let Ok(ids) = load_cached_package_ids(&chain_id).await {
        return Ok(ids);
    }

    let output = Command::new("bash")
        .arg(PUBLISH_SCRIPT_FILE)
        .output()
        .await
        .context("failed to execute publish_package.sh")?;

    let stdout = std::str::from_utf8(&output.stdout).context("publish script stdout is not valid utf-8")?;

    if !output.status.success() {
        let stderr = std::str::from_utf8(&output.stderr).context("publish script stderr is not valid utf-8")?;
        anyhow::bail!("failed to publish move package: \n\n{stdout}\n\n{stderr}");
    }

    let mut audit_trail_package_id = None;
    let mut tf_components_package_id = None;

    for line in stdout.lines() {
        let Some(exported) = line.strip_prefix("export ") else {
            continue;
        };
        let Some((key, value)) = exported.split_once('=') else {
            continue;
        };

        match key {
            "IOTA_AUDIT_TRAIL_PKG_ID" => {
                let package_id =
                    ObjectID::from_str(value).context("failed to parse published audit_trail package ID")?;
                audit_trail_package_id = Some(package_id);
            }
            "IOTA_TF_COMPONENTS_PKG_ID" => {
                let package_id =
                    ObjectID::from_str(value).context("failed to parse published TfComponents package ID")?;
                tf_components_package_id = Some(package_id);
            }
            _ => {}
        }
    }

    let ids = PublishedPackageIds {
        audit_trail_package_id: audit_trail_package_id
            .ok_or_else(|| anyhow!("publish script did not expose IOTA_AUDIT_TRAIL_PKG_ID"))?,
        tf_components_package_id,
    };

    fs::write(
        CACHED_PKG_FILE,
        format!(
            "{};{};{}",
            ids.audit_trail_package_id,
            ids.tf_components_package_id
                .map(|package_id| package_id.to_string())
                .unwrap_or_default(),
            chain_id
        ),
    )
    .await
    .context("failed to write cached package IDs")?;

    Ok(ids)
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
        let package_ids = PACKAGE_IDS
            .get_or_try_init(|| publish_package_ids(&iota_client))
            .await
            .copied()?;

        // Use a dedicated ephemeral signer per test to avoid object-lock contention.
        let signer = InMemSigner::new();
        let signer_address = signer.get_address().await?;
        request_funds(&signer_address).await?;

        let client = AuditTrailClient::from_iota_client(
            iota_client.clone(),
            Some(PackageOverrides {
                audit_trail_package_id: Some(package_ids.audit_trail_package_id),
                tf_components_package_id: package_ids.tf_components_package_id,
                ..PackageOverrides::default()
            }),
        )
        .await?;
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

    /// Creates a trail with the given initial record data and returns its ObjectID.
    pub(crate) async fn create_test_trail(&self, data: Data) -> anyhow::Result<ObjectID> {
        self.create_test_trail_with_tags(data, std::iter::empty::<String>())
            .await
    }

    /// Creates a trail with the given initial record data and available tags.
    pub(crate) async fn create_test_trail_with_tags<I, S>(&self, data: Data, tags: I) -> anyhow::Result<ObjectID>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let created = self
            .create_trail()
            .with_initial_record(InitialRecord::new(data, None, None))
            .with_record_tags(tags)
            .finish()
            .build_and_execute(self)
            .await?
            .output;
        Ok(created.trail_id)
    }

    /// Creates a role on the given trail with the specified permissions.
    pub(crate) async fn create_role(
        &self,
        trail_id: ObjectID,
        role_name: &str,
        permissions: impl IntoIterator<Item = Permission>,
        record_tags: Option<RecordTags>,
    ) -> anyhow::Result<RoleCreated> {
        let created = self
            .trail(trail_id)
            .access()
            .for_role(role_name)
            .create(
                PermissionSet {
                    permissions: permissions.into_iter().collect::<HashSet<_>>(),
                },
                record_tags,
            )
            .build_and_execute(self)
            .await?
            .output;
        Ok(created)
    }

    /// Issues a capability for the given role on the trail.
    pub(crate) async fn issue_cap(
        &self,
        trail_id: ObjectID,
        role_name: &str,
        options: CapabilityIssueOptions,
    ) -> anyhow::Result<CapabilityIssued> {
        let issued = self
            .trail(trail_id)
            .access()
            .for_role(role_name)
            .issue_capability(options)
            .build_and_execute(self)
            .await?
            .output;
        Ok(issued)
    }
}

impl CoreClientReadOnly for TestClient {
    fn package_id(&self) -> ObjectID {
        self.client.package_id()
    }

    fn tf_components_package_id(&self) -> Option<ObjectID> {
        self.client.tf_components_package_id()
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
