// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID, ObjectRef};
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::{Argument, ObjectArg, ProgrammableTransaction};
use iota_interaction::{OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::{Capability, Data};
use crate::core::utils;
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
        Self::build_trail_transaction_for_owner(client, trail_id, owner, "add_record", |ptb, trail_tag| {
            data.ensure_matches_tag(trail_tag)?;

            let data_arg = data.to_ptb(ptb, "stored_data")?;
            let metadata = utils::ptb_pure(ptb, "record_metadata", record_metadata)?;
            let clock = utils::get_clock_ref(ptb);
            Ok(vec![data_arg, metadata, clock])
        })
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
        Self::build_trail_transaction_for_owner(client, trail_id, owner, "delete_record", |ptb, _| {
            let seq = utils::ptb_pure(ptb, "sequence_number", sequence_number)?;
            let clock = utils::get_clock_ref(ptb);
            Ok(vec![seq, clock])
        })
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
        Self::build_read_only_transaction(client, trail_id, "get_record", |ptb| {
            let seq = utils::ptb_pure(ptb, "sequence_number", sequence_number)?;
            Ok(vec![seq])
        })
        .await
    }

    pub(super) async fn has_record<C>(
        client: &C,
        trail_id: ObjectID,
        sequence_number: u64,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_read_only_transaction(client, trail_id, "has_record", |ptb| {
            let seq = utils::ptb_pure(ptb, "sequence_number", sequence_number)?;
            Ok(vec![seq])
        })
        .await
    }

    pub(super) async fn record_count<C>(client: &C, trail_id: ObjectID) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_read_only_transaction(client, trail_id, "record_count", |_| Ok(vec![])).await
    }

    pub(super) async fn first_sequence<C>(client: &C, trail_id: ObjectID) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_read_only_transaction(client, trail_id, "first_sequence", |_| Ok(vec![])).await
    }

    pub(super) async fn last_sequence<C>(client: &C, trail_id: ObjectID) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_read_only_transaction(client, trail_id, "last_sequence", |_| Ok(vec![])).await
    }

    async fn build_trail_transaction_for_owner<C, F>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        method: impl AsRef<str>,
        additional_args: F,
    ) -> Result<ProgrammableTransaction, Error>
    where
        F: FnOnce(&mut ProgrammableTransactionBuilder, &TypeTag) -> Result<Vec<Argument>, Error>,
        C: CoreClientReadOnly + OptionalSync,
    {
        let cap_ref = Self::get_capability_ref(client, owner, trail_id).await?;
        Self::build_trail_transaction_with_cap_ref(client, trail_id, cap_ref, method, additional_args).await
    }

    async fn build_trail_transaction_with_cap_ref<C, F>(
        client: &C,
        trail_id: ObjectID,
        cap_ref: ObjectRef,
        method: impl AsRef<str>,
        additional_args: F,
    ) -> Result<ProgrammableTransaction, Error>
    where
        F: FnOnce(&mut ProgrammableTransactionBuilder, &TypeTag) -> Result<Vec<Argument>, Error>,
        C: CoreClientReadOnly + OptionalSync,
    {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let type_tag = utils::get_type_tag(client, &trail_id).await?;
        let tag = vec![type_tag.clone()];
        let trail_arg = utils::get_shared_object_arg(client, &trail_id, true).await?;

        let mut args = vec![
            ptb.obj(trail_arg)
                .map_err(|e| Error::InvalidArgument(format!("Failed to create trail argument: {e}")))?,
            ptb.obj(ObjectArg::ImmOrOwnedObject(cap_ref))
                .map_err(|e| Error::InvalidArgument(format!("Failed to create cap argument: {e}")))?,
        ];

        args.extend(additional_args(&mut ptb, &type_tag)?);

        let function = iota_interaction::types::Identifier::from_str(method.as_ref())
            .map_err(|e| Error::InvalidArgument(format!("Invalid method name '{}': {e}", method.as_ref())))?;

        ptb.programmable_move_call(client.package_id(), ident_str!("main").into(), function, tag, args);

        Ok(ptb.finish())
    }

    async fn get_capability_ref<C>(client: &C, owner: IotaAddress, trail_id: ObjectID) -> Result<ObjectRef, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let cap: Capability = client
            .find_object_for_address(owner, |cap: &Capability| cap.target_key == trail_id)
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?
            .ok_or_else(|| {
                Error::InvalidArgument(format!("no capability found for owner {owner} and trail {trail_id}"))
            })?;

        let object_id = *cap.id.object_id();
        utils::get_object_ref_by_id(client, &object_id).await
    }

    async fn build_read_only_transaction<C, F>(
        client: &C,
        trail_id: ObjectID,
        method: impl AsRef<str>,
        additional_args: F,
    ) -> Result<ProgrammableTransaction, Error>
    where
        F: FnOnce(&mut ProgrammableTransactionBuilder) -> Result<Vec<Argument>, Error>,
        C: CoreClientReadOnly + OptionalSync,
    {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let tag = vec![utils::get_type_tag(client, &trail_id).await?];
        let trail_arg = utils::get_shared_object_arg(client, &trail_id, false).await?;

        let mut args = vec![
            ptb.obj(trail_arg)
                .map_err(|e| Error::InvalidArgument(format!("Failed to create trail argument: {e}")))?,
        ];

        args.extend(additional_args(&mut ptb)?);

        let function = iota_interaction::types::Identifier::from_str(method.as_ref())
            .map_err(|e| Error::InvalidArgument(format!("Invalid method name '{}': {e}", method.as_ref())))?;

        ptb.programmable_move_call(client.package_id(), ident_str!("main").into(), function, tag, args);

        Ok(ptb.finish())
    }
}
