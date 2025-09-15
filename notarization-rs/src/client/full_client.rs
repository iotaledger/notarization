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
    /// Creates a builder for a locked notarization.
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

    /// Creates a builder for a dynamic notarization.
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
    /// **Important**: The `state` can only  be updated depending on the used `NotarizationMethod`:
    /// - Dynamic: Can be updated anytime after notarization creation
    /// - Locked: Immutable after notarization creation
    ///
    /// Using this function will:
    /// - set the `state` to the `new_state`
    /// - increase the `state_version_count` by 1
    /// - set the `last_state_change_at` timestamp to the current clock timestamp in milliseconds
    /// - emits a `NotarizationUpdated` Move event in case of success
    /// - fail if the notarization uses `NotarizationMethod::Locked`
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

    /// Destroys a notarization permanently.
    ///
    /// The notarization must not have active time locks preventing deletion.
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

    /// Updates the metadata of a notarization.
    ///
    /// **Important**: The `updatable_metadata` can only be updated depending on the used
    /// `NotarizationMethod`:
    /// - Dynamic: Can be updated anytime after notarization creation
    /// - Locked: Immutable after notarization creation
    ///
    /// NOTE:
    /// - does not affect the `state_version_count` or the `last_state_change_at` timestamp
    /// - will fail if the notarization uses the `NotarizationMethod::Locked`
    /// - Only the `updatable_metadata` can be changed; the `immutable_metadata::description`
    ///   remains fixed
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

    /// Transfers ownership of a dynamic notarization.
    ///
    /// The notarization must not have active transfer locks.
    ///
    /// **Important**: Only works on dynamic notarizations.
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
