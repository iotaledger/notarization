// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Record-tag registry APIs for audit trails.

use iota_interaction::types::base_types::ObjectID;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::CoreClient;
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;

use crate::core::trail::AuditTrailFull;

mod operations;
mod transactions;

pub use transactions::{AddRecordTag, RemoveRecordTag};

/// Tag-registry API scoped to a specific trail.
#[derive(Debug, Clone)]
pub struct TrailTags<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
}

impl<'a, C> TrailTags<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self { client, trail_id }
    }

    /// Adds a tag to the trail-owned record-tag registry.
    pub fn add<S>(&self, tag: impl Into<String>) -> TransactionBuilder<AddRecordTag>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(AddRecordTag::new(self.trail_id, owner, tag.into()))
    }

    /// Removes a tag from the trail-owned record-tag registry.
    pub fn remove<S>(&self, tag: impl Into<String>) -> TransactionBuilder<RemoveRecordTag>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(RemoveRecordTag::new(self.trail_id, owner, tag.into()))
    }
}
