use async_trait::async_trait;
use iota_interaction::rpc_types::{IotaExecutionStatus, IotaTransactionBlockEffects, IotaTransactionBlockEffectsAPI};
use iota_interaction::types::id::UID;
use iota_interaction::types::object::Owner;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaTransactionBlockEffectsMutAPI, OptionalSend, OptionalSync};
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

use super::builder::NotarizationBuilder;
use super::operations::{NotarizationImpl, NotarizationOperations};
use super::state::State;
use super::timelock::LockMetadata;
use super::NotarizationMethod;
use crate::error::Error;
use crate::package::notarization_package_id;

/// The immutable metadata of a notarization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ImmutableMetadata {
    /// Timestamp when the `Notarization` was created
    created_at: u64,
    /// Description of the `Notarization`
    description: Option<String>,
    /// Optional lock metadata for `Notarization`
    locking: Option<LockMetadata>,
}

/// A notarization that is stored on the chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnChainNotarization {
    id: UID,
    state: State,
    immutable_metadata: ImmutableMetadata,
    updateable_metadata: Option<String>,
    last_state_change_at: u64,
    state_version_count: u64,
    method: NotarizationMethod,
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

        let operations = NotarizationImpl;

        match method {
            NotarizationMethod::Dynamic => {
                if delete_lock.is_some() {
                    return Err(Error::InvalidArgument(
                        "Delete lock cannot be set for dynamic notarizations".to_string(),
                    ));
                }

                operations.new_dynamic(
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

                operations.new_locked(
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

    async fn apply<C>(
        mut self,
        effects: &mut IotaTransactionBlockEffects,
        client: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        if let IotaExecutionStatus::Failure { error } = effects.status() {
            return Err(Error::TransactionUnexpectedResponse(error.clone()));
        }
        let created_objects = effects
            .created()
            .iter()
            .enumerate()
            .filter(|(_, elem)| matches!(elem.owner, Owner::ObjectOwner(_)))
            .map(|(i, obj)| (i, obj.object_id()));

        // Will try getting the notarization with similiar state and metadata
        let is_target_notarization = |notarization: &OnChainNotarization| {
            let state = self.builder.state.clone().expect("State is required");

            notarization.state == state
                && notarization.immutable_metadata.description == self.builder.immutable_description.clone()
                && notarization.updateable_metadata == self.builder.updateable_metadata.clone()
        };

        let mut target_notarization_pos = None;
        let mut target_notarization = None;
        for (i, obj_id) in created_objects {
            match client.get_object_by_id::<OnChainNotarization>(obj_id).await {
                Ok(notarization) if is_target_notarization(&notarization) => {
                    target_notarization_pos = Some(i);
                    target_notarization = Some(notarization);
                    break;
                }
                _ => continue,
            }
        }

        let (Some(i), Some(notarization)) = (target_notarization_pos, target_notarization) else {
            return Err(Error::TransactionUnexpectedResponse(
                "failed to find the correct notarization in this transaction's effects".to_owned(),
            ));
        };

        effects.created_mut().swap_remove(i);

        Ok(notarization)
    }
}
