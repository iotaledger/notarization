// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Internal helpers that build trail-level programmable transactions.
//!
//! These helpers select the required trail-level permission and encode the corresponding metadata, migration,
//! and deletion calls.

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use crate::core::internal::tx;
use crate::core::types::Permission;
use crate::error::Error;

/// Internal namespace for trail-level transaction construction.
pub(super) struct TrailOps;

impl TrailOps {
    /// Builds the `migrate` call.
    pub(super) async fn migrate<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::Migrate,
            selected_capability_id,
            "migrate",
            |ptb, _| {
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![clock])
            },
        )
        .await
    }

    /// Builds the `update_metadata` call.
    pub(super) async fn update_metadata<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        metadata: Option<String>,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::UpdateMetadata,
            selected_capability_id,
            "update_metadata",
            |ptb, _| {
                let metadata_arg = tx::ptb_pure(ptb, "new_metadata", metadata)?;
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![metadata_arg, clock])
            },
        )
        .await
    }

    /// Builds the `delete_audit_trail` call.
    pub(super) async fn delete_audit_trail<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteAuditTrail,
            selected_capability_id,
            "delete_audit_trail",
            |ptb, _| {
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![clock])
            },
        )
        .await
    }
}
