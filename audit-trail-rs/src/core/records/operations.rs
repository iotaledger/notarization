// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Internal record-operation helpers that build trail-scoped programmable transactions.
//!
//! These helpers enforce the Rust-side preflight checks around record tags and then encode the exact Move call
//! arguments expected by the trail package.

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use crate::core::internal::capability::find_capable_cap_for_tag;
use crate::core::internal::{trail as trail_reader, tx};
use crate::core::types::{Data, Permission};
use crate::error::Error;

/// Internal namespace for record-related transaction construction.
pub(super) struct RecordsOps;

impl RecordsOps {
    /// Builds the `add_record` call.
    ///
    /// Tagged writes are prevalidated against the trail tag registry and require a capability whose role allows
    /// both `AddRecord` and the requested tag.
    pub(super) async fn add_record<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        data: Data,
        record_metadata: Option<String>,
        record_tag: Option<String>,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let package_id = client.package_id();
        if let Some(tag) = record_tag.clone() {
            let trail = trail_reader::get_audit_trail(trail_id, client).await?;
            if !trail.tags.contains_key(&tag) {
                return Err(Error::InvalidArgument(format!(
                    "record tag '{tag}' is not defined for trail {trail_id}"
                )));
            }
            let cap_ref = if let Some(capability_id) = selected_capability_id {
                tx::get_object_ref_by_id(client, &capability_id).await?
            } else {
                find_capable_cap_for_tag(client, owner, trail_id, &trail, &tag).await?
            };

            tx::build_trail_transaction_with_cap_ref(client, trail_id, cap_ref, "add_record", |ptb, trail_tag| {
                data.ensure_matches_tag(trail_tag, package_id)?;

                let data_arg = data.into_ptb(ptb, package_id)?;
                let metadata = tx::ptb_pure(ptb, "record_metadata", record_metadata)?;
                let tag_arg = tx::ptb_pure(ptb, "record_tag", Some(tag))?;
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![data_arg, metadata, tag_arg, clock])
            })
            .await
        } else {
            tx::build_trail_transaction(
                client,
                trail_id,
                owner,
                Permission::AddRecord,
                selected_capability_id,
                "add_record",
                |ptb, trail_tag| {
                    data.ensure_matches_tag(trail_tag, package_id)?;

                    let data_arg = data.into_ptb(ptb, package_id)?;
                    let metadata = tx::ptb_pure(ptb, "record_metadata", record_metadata)?;
                    let tag = tx::ptb_pure(ptb, "record_tag", Option::<String>::None)?;
                    let clock = tx::get_clock_ref(ptb);
                    Ok(vec![data_arg, metadata, tag, clock])
                },
            )
            .await
        }
    }

    /// Builds the `delete_record` call.
    ///
    /// Authorization and locking remain enforced by the Move entry point.
    pub(super) async fn delete_record<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        sequence_number: u64,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteRecord,
            selected_capability_id,
            "delete_record",
            |ptb, _| {
                let seq = tx::ptb_pure(ptb, "sequence_number", sequence_number)?;
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![seq, clock])
            },
        )
        .await
    }

    /// Builds the `delete_records_batch` call.
    ///
    /// Batch deletion requires `DeleteAllRecords` and deletes from the front of the trail.
    pub(super) async fn delete_records_batch<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        limit: u64,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteAllRecords,
            selected_capability_id,
            "delete_records_batch",
            |ptb, _| {
                let limit_arg = tx::ptb_pure(ptb, "limit", limit)?;
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![limit_arg, clock])
            },
        )
        .await
    }

    /// Builds the read-only `get_record` call.
    pub(super) async fn get_record<C>(
        client: &C,
        trail_id: ObjectID,
        sequence_number: u64,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_read_only_transaction(client, trail_id, "get_record", |ptb| {
            let seq = tx::ptb_pure(ptb, "sequence_number", sequence_number)?;
            Ok(vec![seq])
        })
        .await
    }

    /// Builds the read-only `record_count` call.
    pub(super) async fn record_count<C>(client: &C, trail_id: ObjectID) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_read_only_transaction(client, trail_id, "record_count", |_| Ok(vec![])).await
    }
}
