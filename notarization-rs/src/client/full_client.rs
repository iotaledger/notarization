// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Notarization Client
//!
//! The full client provides read-write access to notarizations on the IOTA blockchain.
//!
//! ## Overview
//!
//! This client extends [`NotarizationClientReadOnly`] with transaction capabilities,
//! allowing you to create, update, transfer, and destroy notarizations.
//!
//! ## Transaction Flow
//!
//! All transaction methods return a [`TransactionBuilder`] that follows this pattern:
//!
//! ```rust,ignore
//! # use notarization::client::full_client::NotarizationClient;
//! # use notarization::core::types::State;
//! # use iota_interaction::types::base_types::ObjectID;
//! # async fn example(client: &NotarizationClient<impl secret_storage::Signer<iota_interaction::IotaKeySignature>>) -> Result<(), Box<dyn std::error::Error>> {
//! # let notarization_id = ObjectID::ZERO;
//! // 1. Create the transaction
//! let result = client
//!     .update_state(State::from_string("New data".to_string(), None), notarization_id)
//!     // 2. Configure transaction parameters (all optional)
//!     .with_gas_budget(1_000_000)     // Set custom gas budget
//!     .with_sender(sender_address)     // Override sender address
//!     .with_gas_payment(vec![coin])   // Use specific coins for gas
//!     // 3. Build and execute
//!     .build_and_execute(&client)      // Signs and submits transaction
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Available Configuration Methods
//!
//! The [`TransactionBuilder`] provides these configuration methods:
//! - `with_gas_budget(amount)` - Set gas budget (default: estimated)
//! - `with_gas_payment(coins)` - Use specific coins for gas payment
//! - `with_gas_owner(address)` - Set gas payer (default: sender)
//! - `with_gas_price(price)` - Override gas price (default: network price)
//! - `with_sender(address)` - Override transaction sender
//! - `with_sponsor(callback)` - Have another party pay for gas
//!
//! ## Example: Complete Notarization Workflow
//!
//! ```rust,ignore
//! # use notarization::core::builder::NotarizationBuilder;
//! # use notarization::core::types::{State, TimeLock};
//! # use notarization::client::full_client::NotarizationClient;
//! # async fn example(client: &NotarizationClient<impl secret_storage::Signer<iota_interaction::IotaKeySignature>>) -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Create a dynamic notarization
//! let create_result = client
//!     .create_dynamic_notarization()
//!     .with_string_state("Initial data", Some("Version 1"))
//!     .with_immutable_description("Status Monitor")
//!     .finish()
//!     .build_and_execute(&client)
//!     .await?;
//!
//! let notarization_id = create_result.output;
//!
//! // 2. Update the state
//! client
//!     .update_state(State::from_string("Updated data", Some("Version 2")), notarization_id)
//!     .build_and_execute(&client)
//!     .await?;
//!
//! // 3. Transfer to another owner
//! client
//!     .transfer_notarization(notarization_id, recipient_address)
//!     .build_and_execute(&client)
//!     .await?;
//! # Ok(())
//! # }
//! ```

use std::ops::Deref;

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::crypto::PublicKey;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::network_name::NetworkName;
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;

use super::read_only::NotarizationClientReadOnly;
use crate::core::builder::{Dynamic, Locked, NotarizationBuilder};
use crate::core::transactions::{DestroyNotarization, TransferNotarization, UpdateMetadata, UpdateState};
use crate::core::types::State;
use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;

/// A client for creating and managing notarizations on the IOTA blockchain.
///
/// This client combines read-only capabilities with transaction signing,
/// enabling full interaction with notarizations.
///
/// ## Type Parameter
///
/// - `S`: The signer type that implements [`Signer<IotaKeySignature>`]
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
    /// Creates a new client with signing capabilities.
    ///
    /// ## Parameters
    ///
    /// - `client`: A read-only client for blockchain interaction
    /// - `signer`: A signer for transaction authorization
    ///
    /// ## Errors
    ///
    /// Returns an error if the signer's public key cannot be retrieved.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// # use notarization::client::full_client::{NotarizationClient, NotarizationClientReadOnly};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let read_client = NotarizationClientReadOnly::new(adapter, package_id)?;
    /// let signer = get_signer()?; // Your signer implementation
    /// let client = NotarizationClient::new(read_client, signer).await?;
    /// # Ok(())
    /// # }
    /// ```
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
    /// Creates a builder for a Locked-Notarization.
    ///
    /// A Locked-Notarization is immutable after creation: its `state` and
    /// `updatable_metadata` are fixed for the lifetime of the object. Its
    /// destruction can be gated by a `delete_lock`.
    ///
    /// On execution the resulting transaction transfers the new `Notarization`
    /// object to the sender and emits a `LockedNotarizationCreated` event.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// # use notarization::client::full_client::NotarizationClient;
    /// # use notarization::core::types::TimeLock;
    /// # async fn example(client: &NotarizationClient<impl secret_storage::Signer<iota_interaction::IotaKeySignature>>) -> Result<(), Box<dyn std::error::Error>> {
    /// let result = client
    ///     .create_locked_notarization()
    ///     .with_string_state("Contract v1.0", Some("PDF hash"))
    ///     .with_immutable_description("Employment Agreement")
    ///     .with_delete_lock(TimeLock::UnlockAt(1735689600))
    ///     .finish()?
    ///     .build_and_execute(&client)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See [`NotarizationBuilder<Locked>`] for configuration options.
    pub fn create_locked_notarization(&self) -> NotarizationBuilder<Locked> {
        NotarizationBuilder::locked()
    }

    /// Creates a builder for a Dynamic-Notarization.
    ///
    /// A Dynamic-Notarization can be updated after creation: `state` and
    /// `updatable_metadata` can be replaced via
    /// [`Self::update_state`] and [`Self::update_metadata`], and ownership
    /// can be transferred via [`Self::transfer_notarization`] when the
    /// configured `transfer_lock` permits it.
    ///
    /// On execution the resulting transaction transfers the new `Notarization`
    /// object to the sender and emits a `DynamicNotarizationCreated` event.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// # use notarization::client::full_client::NotarizationClient;
    /// # use notarization::core::types::TimeLock;
    /// # async fn example(client: &NotarizationClient<impl secret_storage::Signer<iota_interaction::IotaKeySignature>>) -> Result<(), Box<dyn std::error::Error>> {
    /// let result = client
    ///     .create_dynamic_notarization()
    ///     .with_string_state("Status: Active", None)
    ///     .with_immutable_description("Service Monitor")
    ///     .with_transfer_lock(TimeLock::None)
    ///     .finish()
    ///     .build_and_execute(&client)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See [`NotarizationBuilder<Dynamic>`] for configuration options.
    pub fn create_dynamic_notarization(&self) -> NotarizationBuilder<Dynamic> {
        NotarizationBuilder::dynamic()
    }
}

impl<S> NotarizationClient<S>
where
    S: Signer<IotaKeySignature> + OptionalSync,
{
    /// Updates the state of a notarization.
    ///
    /// On success the on-chain transaction replaces `state` with `new_state`,
    /// increments `state_version_count` by one, refreshes
    /// `last_state_change_at` to the on-chain clock timestamp (in
    /// milliseconds since the Unix epoch), and emits a `NotarizationUpdated`
    /// Move event.
    ///
    /// Behaviour depends on the Notarization Method:
    /// * `Dynamic`: always permitted — the underlying `update_lock` is fixed to `TimeLock::None`.
    /// * `Locked`: always aborts on-chain, because the underlying `update_lock` is pinned to
    ///   `TimeLock::UntilDestroyed`.
    ///
    /// ## Parameters
    ///
    /// - `new_state`: The new state to set
    /// - `notarization_id`: The ID of the notarization to update
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// # use notarization::client::full_client::NotarizationClient;
    /// # use notarization::core::types::State;
    /// # use iota_interaction::types::base_types::ObjectID;
    /// # async fn example(client: &NotarizationClient<impl secret_storage::Signer<iota_interaction::IotaKeySignature>>, notarization_id: ObjectID) -> Result<(), Box<dyn std::error::Error>> {
    /// client
    ///     .update_state(
    ///         State::from_string("Status: Completed", Some("Final version")),
    ///         notarization_id
    ///     )
    ///     .build_and_execute(&client)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Returns a [`TransactionBuilder`]. See [module docs](self) for transaction flow.
    pub fn update_state(&self, new_state: State, notarization_id: ObjectID) -> TransactionBuilder<UpdateState> {
        TransactionBuilder::new(UpdateState::new(new_state, notarization_id))
    }

    /// Destroys a notarization permanently and releases its object ID.
    ///
    /// All component `TimeLock`s of the attached `LockMetadata` are
    /// destroyed in the process. The notarization must currently be
    /// destroy-allowed (see
    /// [`NotarizationClientReadOnly::is_destroy_allowed`]); otherwise the
    /// on-chain transaction aborts. A `TimeLock::Infinite` lock is not
    /// destructible and therefore always blocks destruction.
    ///
    /// On success the on-chain transaction emits a `NotarizationDestroyed`
    /// event.
    ///
    /// ## Parameters
    ///
    /// - `notarization_id`: The ID of the notarization to destroy
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// # use notarization::client::full_client::NotarizationClient;
    /// # use iota_interaction::types::base_types::ObjectID;
    /// # async fn example(client: &NotarizationClient<impl secret_storage::Signer<iota_interaction::IotaKeySignature>>, notarization_id: ObjectID) -> Result<(), Box<dyn std::error::Error>> {
    /// client
    ///     .destroy(notarization_id)
    ///     .build_and_execute(&client)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Returns a [`TransactionBuilder`]. See [module docs](self) for transaction flow.
    pub fn destroy(&self, notarization_id: ObjectID) -> TransactionBuilder<DestroyNotarization> {
        TransactionBuilder::new(DestroyNotarization::new(notarization_id))
    }

    /// Updates the `updatable_metadata` of a notarization.
    ///
    /// Does not affect `state`, `state_version_count`,
    /// `last_state_change_at`, or the immutable description in
    /// `immutable_metadata`.
    ///
    /// Behaviour depends on the Notarization Method:
    /// * `Dynamic`: always permitted — the underlying `update_lock` is fixed to `TimeLock::None`.
    /// * `Locked`: always aborts on-chain, because the underlying `update_lock` is pinned to
    ///   `TimeLock::UntilDestroyed`.
    ///
    /// ## Parameters
    ///
    /// - `metadata`: The new metadata (or `None` to clear)
    /// - `notarization_id`: The ID of the notarization to update
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// # use notarization::NotarizationClient;
    /// # use iota_interaction::types::base_types::ObjectID;
    /// # async fn example(client: &NotarizationClient<impl secret_storage::Signer<iota_interaction::IotaKeySignature>>, notarization_id: ObjectID) -> Result<(), Box<dyn std::error::Error>> {
    /// client
    ///     .update_metadata(
    ///         Some("Reviewed by legal team".to_string()),
    ///         notarization_id
    ///     )
    ///     .build_and_execute(&client)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Returns a [`TransactionBuilder`]. See [module docs](self) for transaction flow.
    pub fn update_metadata(
        &self,
        metadata: Option<String>,
        notarization_id: ObjectID,
    ) -> TransactionBuilder<UpdateMetadata> {
        TransactionBuilder::new(UpdateMetadata::new(metadata, notarization_id))
    }

    /// Transfers ownership of a notarization to another address.
    ///
    /// Permitted only when the notarization has no `LockMetadata` or when
    /// its `transfer_lock` is not currently active.
    ///
    /// Behaviour depends on the Notarization Method:
    /// * `Dynamic`: on success the notarization is transferred to `recipient`. Submitting while the configured
    ///   `transfer_lock` is engaged aborts on-chain.
    /// * `Locked`: always aborts on-chain — Locked-Notarizations have their `transfer_lock` pinned to
    ///   `TimeLock::UntilDestroyed` and are therefore non-transferable.
    ///
    /// On success the on-chain transaction emits a
    /// `DynamicNotarizationTransferred` event.
    ///
    /// ## Parameters
    ///
    /// - `notarization_id`: The ID of the notarization to transfer
    /// - `recipient`: The address of the new owner
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// # use notarization::client::full_client::NotarizationClient;
    /// # use iota_interaction::types::base_types::{ObjectID, IotaAddress};
    /// # async fn example(client: &NotarizationClient<impl secret_storage::Signer<iota_interaction::IotaKeySignature>>, notarization_id: ObjectID, recipient: IotaAddress) -> Result<(), Box<dyn std::error::Error>> {
    /// client
    ///     .transfer_notarization(notarization_id, recipient)
    ///     .build_and_execute(&client)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Returns a [`TransactionBuilder`]. See [module docs](self) for transaction flow.
    pub fn transfer_notarization(
        &self,
        notarization_id: ObjectID,
        recipient: IotaAddress,
    ) -> TransactionBuilder<TransferNotarization> {
        TransactionBuilder::new(TransferNotarization::new(recipient, notarization_id))
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
