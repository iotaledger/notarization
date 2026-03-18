// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::{Data, OnChainAuditTrail, Permission};
use crate::core::{operations, utils};
use crate::error::Error;

pub(super) struct RecordsOps;

impl RecordsOps {
    pub(super) async fn add_record<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        data: Data,
        record_metadata: Option<String>,
        record_tag: Option<String>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        if let Some(tag) = record_tag.clone() {
            let trail = operations::get_audit_trail(trail_id, client).await?;
            if !trail.tags.contents.iter().any(|allowed_tag| allowed_tag == &tag) {
                return Err(Error::InvalidArgument(format!(
                    "record tag '{tag}' is not defined for trail {trail_id}"
                )));
            }
            let cap_ref = find_capable_cap_for_tag(client, owner, trail_id, &trail, &tag).await?;

            operations::build_trail_transaction_with_cap_ref(
                client,
                trail_id,
                cap_ref,
                "add_record",
                |ptb, trail_tag| {
                    data.ensure_matches_tag(trail_tag)?;

                    let data_arg = data.into_ptb(ptb, "stored_data")?;
                    let metadata = utils::ptb_pure(ptb, "record_metadata", record_metadata)?;
                    let tag_arg = utils::ptb_pure(ptb, "record_tag", Some(tag))?;
                    let clock = utils::get_clock_ref(ptb);
                    Ok(vec![data_arg, metadata, tag_arg, clock])
                },
            )
            .await
        } else {
            operations::build_trail_transaction(
                client,
                trail_id,
                owner,
                Permission::AddRecord,
                "add_record",
                |ptb, trail_tag| {
                    data.ensure_matches_tag(trail_tag)?;

                    let data_arg = data.into_ptb(ptb, "stored_data")?;
                    let metadata = utils::ptb_pure(ptb, "record_metadata", record_metadata)?;
                    let tag = utils::ptb_pure(ptb, "record_tag", Option::<String>::None)?;
                    let clock = utils::get_clock_ref(ptb);
                    Ok(vec![data_arg, metadata, tag, clock])
                },
            )
            .await
        }
    }

    pub(super) async fn delete_record<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        sequence_number: u64,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteRecord,
            "delete_record",
            |ptb, _| {
                let seq = utils::ptb_pure(ptb, "sequence_number", sequence_number)?;
                let clock = utils::get_clock_ref(ptb);
                Ok(vec![seq, clock])
            },
        )
        .await
    }

    pub(super) async fn delete_records_batch<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        limit: u64,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteAllRecords,
            "delete_records_batch",
            |ptb, _| {
                let limit_arg = utils::ptb_pure(ptb, "limit", limit)?;
                let clock = utils::get_clock_ref(ptb);
                Ok(vec![limit_arg, clock])
            },
        )
        .await
    }

    pub(super) async fn get_record<C>(
        client: &C,
        trail_id: ObjectID,
        sequence_number: u64,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_read_only_transaction(client, trail_id, "get_record", |ptb| {
            let seq = utils::ptb_pure(ptb, "sequence_number", sequence_number)?;
            Ok(vec![seq])
        })
        .await
    }

    pub(super) async fn record_count<C>(client: &C, trail_id: ObjectID) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_read_only_transaction(client, trail_id, "record_count", |_| Ok(vec![])).await
    }
}

async fn find_capable_cap_for_tag<C>(
    client: &C,
    owner: IotaAddress,
    trail_id: ObjectID,
    trail: &OnChainAuditTrail,
    tag: &str,
) -> Result<iota_interaction::types::base_types::ObjectRef, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let valid_roles = trail
        .roles
        .roles
        .iter()
        .filter(|(_, role)| {
            role.permissions.contains(&Permission::AddRecord)
                && role.data.as_ref().is_some_and(|record_tags| record_tags.allows(tag))
        })
        .map(|(name, _)| name.clone())
        .collect::<std::collections::HashSet<_>>();

    let cap = client
        .find_object_for_address(owner, |cap: &crate::core::types::Capability| {
            cap.target_key == trail_id && valid_roles.contains(&cap.role)
        })
        .await
        .map_err(|e| Error::RpcError(e.to_string()))?
        .ok_or_else(|| {
            Error::InvalidArgument(format!(
                "no capability with {:?} permission and record tag '{tag}' found for owner {owner} and trail {trail_id}",
                Permission::AddRecord
            ))
        })?;

    let object_id = *cap.id.object_id();
    utils::get_object_ref_by_id(client, &object_id).await
}
