// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use super::AuditTrailImpl;
use crate::core::move_utils;
use crate::core::types::Data;
use crate::error::Error;

impl AuditTrailImpl {
    pub(crate) async fn add_record<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        data: Data,
        record_metadata: Option<String>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_trail_transaction_for_owner(client, trail_id, owner, "add_record", |ptb| {
            let data_arg = match data {
                Data::Bytes(bytes) => move_utils::ptb_pure(ptb, "stored_data", bytes)?,
                Data::Text(text) => move_utils::ptb_pure(ptb, "stored_data", text)?,
            };
            let metadata = move_utils::ptb_pure(ptb, "record_metadata", record_metadata)?;
            let clock = move_utils::get_clock_ref(ptb);
            Ok(vec![data_arg, metadata, clock])
        })
        .await
    }

    pub(crate) async fn delete_record<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        sequence_number: u64,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_trail_transaction_for_owner(client, trail_id, owner, "delete_record", |ptb| {
            let seq = move_utils::ptb_pure(ptb, "sequence_number", sequence_number)?;
            let clock = move_utils::get_clock_ref(ptb);
            Ok(vec![seq, clock])
        })
        .await
    }

    pub(crate) async fn get_record<C>(
        client: &C,
        trail_id: ObjectID,
        sequence_number: u64,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_read_only_transaction(client, trail_id, "get_record", |ptb| {
            let seq = move_utils::ptb_pure(ptb, "sequence_number", sequence_number)?;
            Ok(vec![seq])
        })
        .await
    }

    pub(crate) async fn has_record<C>(
        client: &C,
        trail_id: ObjectID,
        sequence_number: u64,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_read_only_transaction(client, trail_id, "has_record", |ptb| {
            let seq = move_utils::ptb_pure(ptb, "sequence_number", sequence_number)?;
            Ok(vec![seq])
        })
        .await
    }

    pub(crate) async fn record_count<C>(client: &C, trail_id: ObjectID) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_read_only_transaction(client, trail_id, "record_count", |_| Ok(vec![])).await
    }

    pub(crate) async fn first_sequence<C>(client: &C, trail_id: ObjectID) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_read_only_transaction(client, trail_id, "first_sequence", |_| Ok(vec![])).await
    }

    pub(crate) async fn last_sequence<C>(client: &C, trail_id: ObjectID) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_read_only_transaction(client, trail_id, "last_sequence", |_| Ok(vec![])).await
    }
}
