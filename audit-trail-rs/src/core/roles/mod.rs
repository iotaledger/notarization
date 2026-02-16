// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::transaction::transaction_builder::{Transaction, TransactionBuilder};
use secret_storage::Signer;
use tokio::sync::OnceCell;

use crate::core::trail::AuditTrailFull;
use crate::core::types::{
    CapabilityDestroyed, CapabilityIssueOptions, CapabilityIssued, CapabilityRevoked, Event, PermissionSet, RoleCreated,
    RoleDeleted, RoleUpdated,
};
use crate::error::Error;

mod operations;
use self::operations::RolesOps;
use iota_interaction::rpc_types::{IotaTransactionBlockEffects, IotaTransactionBlockEvents};

#[derive(Debug, Clone)]
pub struct TrailRoles<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
}

impl<'a, C> TrailRoles<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self { client, trail_id }
    }

    /// Returns a handle bound to a specific role name.
    pub fn for_role(&self, name: impl Into<String>) -> RoleHandle<'a, C> {
        RoleHandle::new(self.client, self.trail_id, name.into())
    }

    /// Creates a new role with the provided permissions.
    pub fn create<S>(
        &self,
        name: impl Into<String>,
        permissions: PermissionSet,
    ) -> Result<TransactionBuilder<CreateRole>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(CreateRole::new(
            self.trail_id,
            owner,
            name.into(),
            permissions,
        )))
    }

    /// Updates permissions for an existing role.
    pub fn update<S>(
        &self,
        name: impl Into<String>,
        permissions: PermissionSet,
    ) -> Result<TransactionBuilder<UpdateRole>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(UpdateRole::new(
            self.trail_id,
            owner,
            name.into(),
            permissions,
        )))
    }

    /// Deletes an existing role.
    pub fn delete<S>(&self, name: impl Into<String>) -> Result<TransactionBuilder<DeleteRole>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(DeleteRole::new(
            self.trail_id,
            owner,
            name.into(),
        )))
    }

    /// Revokes an issued capability.
    pub fn revoke_capability<S>(&self, capability_id: ObjectID) -> Result<TransactionBuilder<RevokeCapability>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(RevokeCapability::new(
            self.trail_id,
            owner,
            capability_id,
        )))
    }

    /// Destroys a capability object.
    pub fn destroy_capability<S>(&self, capability_id: ObjectID) -> Result<TransactionBuilder<DestroyCapability>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(DestroyCapability::new(
            self.trail_id,
            owner,
            capability_id,
        )))
    }
}

#[derive(Debug, Clone)]
pub struct RoleHandle<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
    pub(crate) name: String,
}

impl<'a, C> RoleHandle<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID, name: String) -> Self {
        Self { client, trail_id, name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Issues a capability for this role using optional restrictions.
    pub fn issue_capability<S>(
        &self,
        options: CapabilityIssueOptions,
    ) -> Result<TransactionBuilder<IssueCapability>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(IssueCapability::new(
            self.trail_id,
            owner,
            self.name.clone(),
            options,
        )))
    }

    /// Updates permissions for this role.
    pub fn update_permissions<S>(&self, permissions: PermissionSet) -> Result<TransactionBuilder<UpdateRole>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(UpdateRole::new(
            self.trail_id,
            owner,
            self.name.clone(),
            permissions,
        )))
    }

    /// Deletes this role.
    pub fn delete<S>(&self) -> Result<TransactionBuilder<DeleteRole>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(DeleteRole::new(
            self.trail_id,
            owner,
            self.name.clone(),
        )))
    }
}

#[derive(Debug, Clone)]
pub struct CreateRole {
    trail_id: ObjectID,
    owner: IotaAddress,
    name: String,
    permissions: PermissionSet,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl CreateRole {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, name: String, permissions: PermissionSet) -> Self {
        Self {
            trail_id,
            owner,
            name,
            permissions,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RolesOps::create_role(
            client,
            self.trail_id,
            self.owner,
            self.name.clone(),
            self.permissions.clone(),
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
        for data in &events.data {
            if let Ok(event) = serde_json::from_value::<Event<RoleCreated>>(data.parsed_json.clone()) {
                return Ok(event.data);
            }
        }

        Err(Error::UnexpectedApiResponse("RoleCreated event not found".to_string()))
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Err(Error::UnexpectedApiResponse(
            "RoleCreated output requires transaction events".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct UpdateRole {
    trail_id: ObjectID,
    owner: IotaAddress,
    name: String,
    permissions: PermissionSet,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateRole {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, name: String, permissions: PermissionSet) -> Self {
        Self {
            trail_id,
            owner,
            name,
            permissions,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RolesOps::update_role(
            client,
            self.trail_id,
            self.owner,
            self.name.clone(),
            self.permissions.clone(),
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
        for data in &events.data {
            if let Ok(event) = serde_json::from_value::<Event<RoleUpdated>>(data.parsed_json.clone()) {
                return Ok(event.data);
            }
        }

        Err(Error::UnexpectedApiResponse("RoleUpdated event not found".to_string()))
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Err(Error::UnexpectedApiResponse(
            "RoleUpdated output requires transaction events".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct DeleteRole {
    trail_id: ObjectID,
    owner: IotaAddress,
    name: String,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DeleteRole {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, name: String) -> Self {
        Self {
            trail_id,
            owner,
            name,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RolesOps::delete_role(client, self.trail_id, self.owner, self.name.clone()).await
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
        _: &mut iota_interaction::rpc_types::IotaTransactionBlockEffects,
        events: &mut iota_interaction::rpc_types::IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        for data in &events.data {
            if let Ok(event) = serde_json::from_value::<Event<RoleDeleted>>(data.parsed_json.clone()) {
                return Ok(event.data);
            }
        }

        Err(Error::UnexpectedApiResponse("RoleDeleted event not found".to_string()))
    }

    async fn apply<C>(
        mut self,
        _: &mut iota_interaction::rpc_types::IotaTransactionBlockEffects,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Err(Error::UnexpectedApiResponse(
            "RoleDeleted output requires transaction events".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct IssueCapability {
    trail_id: ObjectID,
    owner: IotaAddress,
    role: String,
    options: CapabilityIssueOptions,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl IssueCapability {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, role: String, options: CapabilityIssueOptions) -> Self {
        Self {
            trail_id,
            owner,
            role,
            options,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RolesOps::issue_capability(
            client,
            self.trail_id,
            self.owner,
            self.role.clone(),
            self.options.clone(),
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
        _: &mut iota_interaction::rpc_types::IotaTransactionBlockEffects,
        events: &mut iota_interaction::rpc_types::IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        for data in &events.data {
            if let Ok(event) = serde_json::from_value::<Event<CapabilityIssued>>(data.parsed_json.clone()) {
                return Ok(event.data);
            }
        }

        Err(Error::UnexpectedApiResponse(
            "CapabilityIssued event not found".to_string(),
        ))
    }

    async fn apply<C>(
        mut self,
        _: &mut iota_interaction::rpc_types::IotaTransactionBlockEffects,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Err(Error::UnexpectedApiResponse(
            "CapabilityIssued output requires transaction events".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct RevokeCapability {
    trail_id: ObjectID,
    owner: IotaAddress,
    capability_id: ObjectID,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl RevokeCapability {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, capability_id: ObjectID) -> Self {
        Self {
            trail_id,
            owner,
            capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RolesOps::revoke_capability(client, self.trail_id, self.owner, self.capability_id).await
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
        _: &mut iota_interaction::rpc_types::IotaTransactionBlockEffects,
        events: &mut iota_interaction::rpc_types::IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        for data in &events.data {
            if let Ok(event) = serde_json::from_value::<Event<CapabilityRevoked>>(data.parsed_json.clone()) {
                return Ok(event.data);
            }
        }

        Err(Error::UnexpectedApiResponse(
            "CapabilityRevoked event not found".to_string(),
        ))
    }

    async fn apply<C>(
        mut self,
        _: &mut iota_interaction::rpc_types::IotaTransactionBlockEffects,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Err(Error::UnexpectedApiResponse(
            "CapabilityRevoked output requires transaction events".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct DestroyCapability {
    trail_id: ObjectID,
    owner: IotaAddress,
    capability_id: ObjectID,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DestroyCapability {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, capability_id: ObjectID) -> Self {
        Self {
            trail_id,
            owner,
            capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RolesOps::destroy_capability(client, self.trail_id, self.owner, self.capability_id).await
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
        _: &mut iota_interaction::rpc_types::IotaTransactionBlockEffects,
        events: &mut iota_interaction::rpc_types::IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        for data in &events.data {
            if let Ok(event) = serde_json::from_value::<Event<CapabilityDestroyed>>(data.parsed_json.clone()) {
                return Ok(event.data);
            }
        }

        Err(Error::UnexpectedApiResponse(
            "CapabilityDestroyed event not found".to_string(),
        ))
    }

    async fn apply<C>(
        mut self,
        _: &mut iota_interaction::rpc_types::IotaTransactionBlockEffects,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Err(Error::UnexpectedApiResponse(
            "CapabilityDestroyed output requires transaction events".to_string(),
        ))
    }
}
