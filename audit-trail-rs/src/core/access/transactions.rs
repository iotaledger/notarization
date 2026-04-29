// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Transaction payloads for audit-trail role and capability administration.
//!
//! These types cache the generated programmable transaction, delegate PTB construction to
//! [`super::operations::AccessOps`], and decode the matching Move events into typed Rust outputs.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::{IotaTransactionBlockEffects, IotaTransactionBlockEvents};
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::operations::AccessOps;
use crate::core::types::{
    CapabilityDestroyed, CapabilityIssueOptions, CapabilityIssued, CapabilityRevoked, Event, PermissionSet,
    RawRoleCreated, RawRoleDeleted, RawRoleUpdated, RoleCreated, RoleDeleted, RoleTags, RoleUpdated,
};
use crate::error::Error;

// ===== CreateRole =====

/// Transaction that creates a role on a trail.
///
/// This maps to the audit-trail `create_role` Move entry point and therefore requires an authorization
/// capability with `AddRoles`.
#[derive(Debug, Clone)]
pub struct CreateRole {
    trail_id: ObjectID,
    owner: IotaAddress,
    name: String,
    permissions: PermissionSet,
    role_tags: Option<RoleTags>,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl CreateRole {
    /// Creates a `CreateRole` transaction builder payload.
    ///
    /// `role_tags`, when present, are serialized as Move `record_tags::RoleTags` role data.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        name: String,
        permissions: PermissionSet,
        role_tags: Option<RoleTags>,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            name,
            permissions,
            role_tags,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AccessOps::create_role(
            client,
            self.trail_id,
            self.owner,
            self.name.clone(),
            self.permissions.clone(),
            self.role_tags.clone(),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for CreateRole {
    type Error = Error;
    type Output = RoleCreated;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| bcs::from_bytes::<RawRoleCreated>(data.bcs.bytes()).ok().map(Into::into))
            .ok_or_else(|| Error::UnexpectedApiResponse("RoleCreated event not found".to_string()))?;

        Ok(event)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!("RoleCreated output requires transaction events")
    }
}

/// Transaction that updates an existing role.
///
/// This updates both the permission set and the optional role-tag data stored for the role. The entry point
/// requires `UpdateRoles`.
#[derive(Debug, Clone)]
pub struct UpdateRole {
    trail_id: ObjectID,
    owner: IotaAddress,
    name: String,
    permissions: PermissionSet,
    role_tags: Option<RoleTags>,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateRole {
    /// Creates an `UpdateRole` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        name: String,
        permissions: PermissionSet,
        role_tags: Option<RoleTags>,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            name,
            permissions,
            role_tags,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AccessOps::update_role(
            client,
            self.trail_id,
            self.owner,
            self.name.clone(),
            self.permissions.clone(),
            self.role_tags.clone(),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for UpdateRole {
    type Error = Error;
    type Output = RoleUpdated;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| bcs::from_bytes::<RawRoleUpdated>(data.bcs.bytes()).ok().map(Into::into))
            .ok_or_else(|| Error::UnexpectedApiResponse("RoleUpdated event not found".to_string()))?;

        Ok(event)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

/// Transaction that deletes a role.
///
/// The reserved initial-admin role cannot be deleted even if the caller holds `DeleteRoles`.
#[derive(Debug, Clone)]
pub struct DeleteRole {
    trail_id: ObjectID,
    owner: IotaAddress,
    name: String,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DeleteRole {
    /// Creates a `DeleteRole` transaction builder payload.
    pub fn new(trail_id: ObjectID, owner: IotaAddress, name: String, selected_capability_id: Option<ObjectID>) -> Self {
        Self {
            trail_id,
            owner,
            name,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AccessOps::delete_role(
            client,
            self.trail_id,
            self.owner,
            self.name.clone(),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for DeleteRole {
    type Error = Error;
    type Output = RoleDeleted;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| bcs::from_bytes::<RawRoleDeleted>(data.bcs.bytes()).ok().map(Into::into))
            .ok_or_else(|| Error::UnexpectedApiResponse("RoleDeleted event not found".to_string()))?;

        Ok(event)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

/// Transaction that issues a capability for a role.
///
/// This mints a new capability object for `role` against `trail_id`. Optional issuance restrictions are
/// copied into the capability object and later enforced on-chain.
#[derive(Debug, Clone)]
pub struct IssueCapability {
    trail_id: ObjectID,
    owner: IotaAddress,
    role: String,
    options: CapabilityIssueOptions,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl IssueCapability {
    /// Creates an `IssueCapability` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        role: String,
        options: CapabilityIssueOptions,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            role,
            options,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AccessOps::issue_capability(
            client,
            self.trail_id,
            self.owner,
            self.role.clone(),
            self.options.clone(),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for IssueCapability {
    type Error = Error;
    type Output = CapabilityIssued;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| serde_json::from_value::<Event<CapabilityIssued>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("CapabilityIssued event not found".to_string()))?;

        Ok(event.data)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

/// Transaction that revokes a capability.
///
/// Revocation writes the capability ID into the trail's revoked-capability denylist. Supplying
/// `capability_valid_until` preserves the same expiry boundary later used by denylist cleanup.
#[derive(Debug, Clone)]
pub struct RevokeCapability {
    trail_id: ObjectID,
    owner: IotaAddress,
    capability_id: ObjectID,
    capability_valid_until: Option<u64>,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl RevokeCapability {
    /// Creates a `RevokeCapability` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        capability_id: ObjectID,
        capability_valid_until: Option<u64>,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            capability_id,
            capability_valid_until,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AccessOps::revoke_capability(
            client,
            self.trail_id,
            self.owner,
            self.capability_id,
            self.capability_valid_until,
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for RevokeCapability {
    type Error = Error;
    type Output = CapabilityRevoked;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| serde_json::from_value::<Event<CapabilityRevoked>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("CapabilityRevoked event not found".to_string()))?;

        Ok(event.data)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

/// Transaction that destroys a capability object.
///
/// This path is for ordinary capabilities. Initial-admin capabilities must use
/// [`DestroyInitialAdminCapability`] instead.
#[derive(Debug, Clone)]
pub struct DestroyCapability {
    trail_id: ObjectID,
    owner: IotaAddress,
    capability_id: ObjectID,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DestroyCapability {
    /// Creates a `DestroyCapability` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        capability_id: ObjectID,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            capability_id,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AccessOps::destroy_capability(
            client,
            self.trail_id,
            self.owner,
            self.capability_id,
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for DestroyCapability {
    type Error = Error;
    type Output = CapabilityDestroyed;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| serde_json::from_value::<Event<CapabilityDestroyed>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("CapabilityDestroyed event not found".to_string()))?;

        Ok(event.data)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

// ===== DestroyInitialAdminCapability =====

/// Transaction that destroys an initial-admin capability without an auth capability.
///
/// Initial-admin capability IDs are tracked separately and cannot be removed through the generic destroy path.
#[derive(Debug, Clone)]
pub struct DestroyInitialAdminCapability {
    trail_id: ObjectID,
    capability_id: ObjectID,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DestroyInitialAdminCapability {
    /// Creates a `DestroyInitialAdminCapability` transaction builder payload.
    pub fn new(trail_id: ObjectID, capability_id: ObjectID) -> Self {
        Self {
            trail_id,
            capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AccessOps::destroy_initial_admin_capability(client, self.trail_id, self.capability_id).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for DestroyInitialAdminCapability {
    type Error = Error;
    type Output = CapabilityDestroyed;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| serde_json::from_value::<Event<CapabilityDestroyed>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("CapabilityDestroyed event not found".to_string()))?;

        Ok(event.data)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

// ===== RevokeInitialAdminCapability =====

/// Transaction that revokes an initial-admin capability.
///
/// This is the dedicated revoke path for capability IDs recognized as active initial-admin capabilities.
#[derive(Debug, Clone)]
pub struct RevokeInitialAdminCapability {
    trail_id: ObjectID,
    owner: IotaAddress,
    capability_id: ObjectID,
    capability_valid_until: Option<u64>,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl RevokeInitialAdminCapability {
    /// Creates a `RevokeInitialAdminCapability` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        capability_id: ObjectID,
        capability_valid_until: Option<u64>,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            capability_id,
            capability_valid_until,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AccessOps::revoke_initial_admin_capability(
            client,
            self.trail_id,
            self.owner,
            self.capability_id,
            self.capability_valid_until,
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for RevokeInitialAdminCapability {
    type Error = Error;
    type Output = CapabilityRevoked;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| serde_json::from_value::<Event<CapabilityRevoked>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("CapabilityRevoked event not found".to_string()))?;

        Ok(event.data)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

/// Transaction that cleans up expired revoked-capability entries.
///
/// This does not revoke additional capabilities. It only prunes denylist entries whose stored expiry has
/// already elapsed.
#[derive(Debug, Clone)]
pub struct CleanupRevokedCapabilities {
    trail_id: ObjectID,
    owner: IotaAddress,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl CleanupRevokedCapabilities {
    /// Creates a `CleanupRevokedCapabilities` transaction builder payload.
    pub fn new(trail_id: ObjectID, owner: IotaAddress, selected_capability_id: Option<ObjectID>) -> Self {
        Self {
            trail_id,
            owner,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AccessOps::cleanup_revoked_capabilities(client, self.trail_id, self.owner, self.selected_capability_id).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for CleanupRevokedCapabilities {
    type Error = Error;
    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply<C>(self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}
