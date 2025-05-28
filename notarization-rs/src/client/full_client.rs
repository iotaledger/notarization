use std::ops::Deref;

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::crypto::PublicKey;
use iota_interaction::{IotaKeySignature, OptionalSync};
use iota_interaction_rust::IotaClientAdapter;
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::network_name::NetworkName;
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;

use super::read_only::NotarizationClientReadOnly;
use crate::core::builder::{Dynamic, Locked, NotarizationBuilder};
use crate::core::destroy::DestroyNotarization;
use crate::core::metadata::UpdateMetadata;
use crate::core::state::{State, UpdateState};
use crate::core::transfer::TransferNotarization;
use crate::error::Error;

/// A client for interacting with the IOTA network.
#[derive(Clone)]
pub struct NotarizationClient<S> {
    /// [`NotarizationClientReadOnly`] instance, used for read-only operations.
    read_client: NotarizationClientReadOnly,
    /// The public key of the client.
    public_key: PublicKey,
    /// The signer of the client.
    signer: S,
}

impl<S> Deref for NotarizationClient<S> {
    type Target = NotarizationClientReadOnly;
    fn deref(&self) -> &Self::Target {
        &self.read_client
    }
}

impl<S> NotarizationClient<S>
where
    S: Signer<IotaKeySignature>,
{
    /// Create a new [`NotarizationClient`].
    pub async fn new(client: NotarizationClientReadOnly, signer: S) -> Result<Self, Error> {
        let public_key = signer
            .public_key()
            .await
            .map_err(|e| Error::InvalidKey(e.to_string()))?;

        Ok(Self {
            public_key,
            read_client: client,
            signer,
        })
    }
}

impl<S> NotarizationClient<S> {
    /// Creates a builder for a locked notarization.
    ///
    /// A locked notarization is immutable once created and requires a delete lock
    /// to be specified. This type of notarization cannot have transfer locks and
    /// provides the highest level of data integrity guarantees.
    ///
    /// # Returns
    ///
    /// A [`NotarizationBuilder<Locked>`] that can be used to configure and create
    /// a locked notarization.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = client.create_locked_notarization()
    ///     .with_state(my_state)
    ///     .with_description("Important document")
    ///     .with_delete_lock(delete_lock);
    /// ```
    pub fn create_locked_notarization(&self) -> NotarizationBuilder<Locked> {
        NotarizationBuilder::locked()
    }

    /// Creates a builder for a dynamic notarization.
    ///
    /// A dynamic notarization allows for updates to its metadata and state after
    /// creation. It supports optional transfer locks to control ownership
    /// changes but cannot have delete locks. This provides flexibility for
    /// evolving data while maintaining notarization integrity.
    ///
    /// # Returns
    ///
    /// A [`NotarizationBuilder<Dynamic>`] that can be used to configure and
    /// create a dynamic notarization.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let builder = client.create_dynamic_notarization()
    ///     .with_state(my_state)
    ///     .with_description("Evolving document")
    ///     .with_transfer_lock(transfer_lock);
    /// ```
    pub fn create_dynamic_notarization(&self) -> NotarizationBuilder<Dynamic> {
        NotarizationBuilder::dynamic()
    }
}

impl<S> NotarizationClient<S>
where
    S: Signer<IotaKeySignature> + OptionalSync,
{
    /// Creates a transaction that updates the state of a notarization
    pub fn update_state(&self, state: State, object_id: ObjectID) -> TransactionBuilder<UpdateState> {
        TransactionBuilder::new(UpdateState::new(state, object_id))
    }

    /// Creates a transaction that destroys a notarization
    pub fn destroy(&self, object_id: ObjectID) -> TransactionBuilder<DestroyNotarization> {
        TransactionBuilder::new(DestroyNotarization::new(object_id))
    }

    /// Creates a transaction that updates the metadata of a notarization
    pub fn update_metadata(&self, metadata: Option<String>, object_id: ObjectID) -> TransactionBuilder<UpdateMetadata> {
        TransactionBuilder::new(UpdateMetadata::new(metadata, object_id))
    }

    /// Creates a transaction that transfers a notarization to a new owner
    pub fn transfer_notarization(
        &self,
        object_id: ObjectID,
        recipient: IotaAddress,
    ) -> TransactionBuilder<TransferNotarization> {
        TransactionBuilder::new(TransferNotarization::new(recipient, object_id))
    }
}

impl<S> CoreClientReadOnly for NotarizationClient<S>
where
    S: OptionalSync,
{
    fn client_adapter(&self) -> &IotaClientAdapter {
        &self.read_client
    }

    fn package_id(&self) -> ObjectID {
        self.read_client.package_id()
    }

    fn network_name(&self) -> &NetworkName {
        self.read_client.network()
    }
}

impl<S> CoreClient<S> for NotarizationClient<S>
where
    S: Signer<IotaKeySignature> + OptionalSync,
{
    fn sender_address(&self) -> IotaAddress {
        IotaAddress::from(&self.public_key)
    }

    fn signer(&self) -> &S {
        &self.signer
    }

    fn sender_public_key(&self) -> &PublicKey {
        &self.public_key
    }
}
