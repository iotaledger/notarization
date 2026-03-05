// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::{Data, Permission};
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
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::AddRecord,
            "add_record",
            |ptb, trail_tag| {
                data.ensure_matches_tag(trail_tag)?;

                let data_arg = data.to_ptb(ptb, "stored_data")?;
                let metadata = utils::ptb_pure(ptb, "record_metadata", record_metadata)?;
                let clock = utils::get_clock_ref(ptb);
                Ok(vec![data_arg, metadata, clock])
            },
        )
        .await
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
