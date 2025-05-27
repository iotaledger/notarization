// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::fmt;
use std::str::FromStr;

use async_trait::async_trait;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::{Argument, ProgrammableTransaction};
use iota_interaction::types::{TypeTag, MOVE_STDLIB_PACKAGE_ID};
use iota_interaction::{ident_str, MoveType, OptionalSync};
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use serde::{Deserialize, Deserializer, Serialize};
use tokio::sync::OnceCell;

use super::move_utils;
use super::operations::{NotarizationImpl, NotarizationOperations};
use crate::error::Error;
use crate::package::notarization_package_id;

/// The state of the `Notarization` that can be updated
#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct State<T = Data> {
    pub data: T,
    #[serde(default)]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Data {
    Bytes(Vec<u8>),
    Text(String),
}

impl<'de> Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_bytes = Vec::<u8>::deserialize(deserializer)?;

        if let Ok(text) = String::from_utf8(raw_bytes.clone()) {
            return Ok(Data::Text(text));
        }

        Ok(Data::Bytes(raw_bytes))
    }
}

impl Data {
    pub(crate) fn tag(&self) -> TypeTag {
        match self {
            Data::Bytes(_) => TypeTag::Vector(Box::new(TypeTag::U8)),
            Data::Text(_) => TypeTag::from_str(&format!("{MOVE_STDLIB_PACKAGE_ID}::string::String"))
                .expect("should be valid type tag"),
        }
    }

    pub fn as_bytes(self) -> Result<Vec<u8>, Error> {
        match self {
            Data::Bytes(data) => Ok(data),
            Data::Text(_) => Err(Error::GenericError("Data is not a vector".to_string())),
        }
    }

    pub fn as_text(self) -> Result<String, Error> {
        match self {
            Data::Bytes(_) => Err(Error::GenericError("Data is not a string".to_string())),
            Data::Text(data) => Ok(data),
        }
    }
}

impl State {
    pub fn data(&self) -> &Data {
        &self.data
    }

    pub fn metadata(&self) -> &Option<String> {
        &self.metadata
    }

    pub fn from_bytes(data: Vec<u8>, metadata: Option<String>) -> Self {
        Self {
            data: Data::Bytes(data),
            metadata,
        }
    }

    pub fn from_string(data: String, metadata: Option<String>) -> Self {
        Self {
            data: Data::Text(data),
            metadata,
        }
    }

    /// Creates a new `Argument` from the `State`.
    ///
    /// To be used when creating a new `Notarization` object on the ledger.
    pub(super) fn into_ptb(
        self,
        ptb: &mut ProgrammableTransactionBuilder,
        package_id: ObjectID,
    ) -> Result<Argument, Error> {
        match self.data {
            Data::Bytes(data) => new_from_vector(ptb, data, self.metadata, package_id),
            Data::Text(data) => new_from_string(ptb, data, self.metadata, package_id),
        }
    }
}

pub(crate) fn new_from_vector(
    ptb: &mut ProgrammableTransactionBuilder,
    data: Vec<u8>,
    metadata: Option<String>,
    package_id: ObjectID,
) -> Result<Argument, Error> {
    let data = move_utils::ptb_pure(ptb, "data", data)?;
    let metadata = move_utils::new_move_option_string(metadata, ptb)?;

    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("notarization").into(),
        ident_str!("new_state_from_vector").into(),
        vec![],
        vec![data, metadata],
    ))
}

pub(crate) fn new_from_string(
    ptb: &mut ProgrammableTransactionBuilder,
    data: String,
    metadata: Option<String>,
    package_id: ObjectID,
) -> Result<Argument, Error> {
    let data = move_utils::new_move_string(data, ptb)?;
    let metadata = move_utils::new_move_option_string(metadata, ptb)?;

    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("notarization").into(),
        ident_str!("new_state_from_string").into(),
        vec![],
        vec![data, metadata],
    ))
}

/// A transaction that updates the state of a notarization
pub struct UpdateState {
    state: State,
    object_id: ObjectID,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateState {
    pub fn new(state: State, object_id: ObjectID) -> Self {
        Self {
            state,
            object_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let package_id = notarization_package_id(client).await?;
        let new_state = self.state.clone();

        NotarizationImpl::update_state(client, package_id, self.object_id, new_state).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for UpdateState {
    type Error = Error;

    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}
