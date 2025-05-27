use async_trait::async_trait;
use iota_interaction::rpc_types::{
    IotaExecutionStatus, IotaTransactionBlockEffects, IotaTransactionBlockEffectsAPI, IotaTransactionBlockEvents,
};
use iota_interaction::types::id::UID;
use iota_interaction::types::object::Owner;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaTransactionBlockEffectsMutAPI, OptionalSend, OptionalSync};
use iota_interaction::rpc_types::{IotaData as _, IotaObjectDataOptions};
use iota_interaction::types::base_types::ObjectID;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_number_from_string;
use serde_json::Value;
use tokio::sync::OnceCell;

use super::builder::NotarizationBuilder;
use super::event::{DynamicNotarizationCreated, Event, LockedNotarizationCreated};
use super::metadata::ImmutableMetadata;
use super::operations::{NotarizationImpl, NotarizationOperations};
use super::state::{Data, State};
use super::timelock::LockMetadata;
use super::NotarizationMethod;
use crate::error::Error;
use crate::package::notarization_package_id;
use iota_interaction::IotaClientTrait;

/// A notarization that is stored on the chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnChainNotarization {
    pub id: UID,
    pub state: State,
    pub immutable_metadata: ImmutableMetadata,
    pub updateable_metadata: Option<String>,
    pub last_state_change_at: u64,
    pub state_version_count: u64,
    pub method: NotarizationMethod,
}

#[derive(Debug, Clone)]
pub struct CreateNotarization<M> {
    builder: NotarizationBuilder<M>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl<M: Clone> CreateNotarization<M> {
    /// Creates a new [`CreateNotarization`] instance.
    pub fn new(builder: NotarizationBuilder<M>) -> Self {
        Self {
            builder,
            cached_ptb: OnceCell::new(),
        }
    }

    /// Makes a [`ProgrammableTransaction`] for the [`CreateNotarization`] instance.
    async fn make_ptb(&self, client: &impl CoreClientReadOnly) -> Result<ProgrammableTransaction, Error> {
        let NotarizationBuilder {
            state,
            immutable_description,
            updateable_metadata,
            method,
            delete_lock,
            transfer_lock,
            ..
        } = self.builder.clone();

        let package_id = notarization_package_id(client).await?;

        let state = state.ok_or_else(|| Error::InvalidArgument("State is required".to_string()))?;

        match method {
            NotarizationMethod::Dynamic => {
                if delete_lock.is_some() {
                    return Err(Error::InvalidArgument(
                        "Delete lock cannot be set for dynamic notarizations".to_string(),
                    ));
                }

                NotarizationImpl::new_dynamic(
                    package_id,
                    state,
                    immutable_description,
                    updateable_metadata,
                    transfer_lock,
                )
            }
            NotarizationMethod::Locked => {
                if transfer_lock.is_some() {
                    return Err(Error::InvalidArgument(
                        "Transfer lock cannot be set for locked notarizations".to_string(),
                    ));
                }

                let delete_lock = delete_lock.ok_or_else(|| {
                    Error::InvalidArgument("Delete lock is required for locked notarizations".to_string())
                })?;

                NotarizationImpl::new_locked(
                    package_id,
                    state,
                    immutable_description,
                    updateable_metadata,
                    delete_lock,
                )
            }
        }
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl<M: Clone + OptionalSend + OptionalSync> Transaction for CreateNotarization<M> {
    type Error = Error;

    type Output = OnChainNotarization;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: Option<IotaTransactionBlockEvents>,
        client: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let events =
            events.ok_or_else(|| Error::TransactionUnexpectedResponse("events should be provided".to_string()))?;

        let method = self.builder.method.clone();

        let data = events
            .data
            .first()
            .ok_or_else(|| Error::TransactionUnexpectedResponse("events should be provided".to_string()))?;

        let notarization_id = match method {
            NotarizationMethod::Dynamic => {
                let event: Event<DynamicNotarizationCreated> = serde_json::from_value(data.parsed_json.clone())
                    .map_err(|e| Error::TransactionUnexpectedResponse(format!("failed to parse event: {}", e)))?;

                event.data.notarization_id
            }
            NotarizationMethod::Locked => {
                let event: Event<LockedNotarizationCreated> = serde_json::from_value(data.parsed_json.clone())
                    .map_err(|e| Error::TransactionUnexpectedResponse(format!("failed to parse event: {}", e)))?;

                event.data.notarization_id
            }
        };

        let notarization = get_object_ref_by_id_with_bcs::<OnChainNotarization>(client, &notarization_id)
            .await
            .map_err(|e| Error::ObjectLookup(e.to_string()))?;

        Ok(notarization)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

pub(crate) async fn get_object_ref_by_id_with_bcs<T: DeserializeOwned>(
    client: &impl CoreClientReadOnly,
    object_id: &ObjectID,
) -> Result<T, Error> {
    let notarization = client
        .client_adapter()
        .read_api()
        .get_object_with_options(*object_id, IotaObjectDataOptions::bcs_lossless())
        .await
        .map_err(|err| Error::ObjectLookup(err.to_string()))?
        .data
        .ok_or_else(|| Error::ObjectLookup("missing data in response".to_string()))?
        .bcs
        .ok_or_else(|| Error::ObjectLookup("missing object content in data".to_string()))?
        .try_into_move()
        .ok_or_else(|| Error::ObjectLookup("failed to convert data to move object".to_string()))?
        .deserialize()
        .map_err(|err| Error::ObjectLookup(err.to_string()))?;

    Ok(notarization)
}
