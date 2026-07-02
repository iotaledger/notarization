// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::id::UID;
use iota_sdk_types::Address;
use serde::{Deserialize, Serialize};

use super::NotarizationMethod;
use super::metadata::ImmutableMetadata;
use super::state::State;

/// A notarization record stored on the blockchain.
///
/// Stores user-defined data together with immutable provenance, optional
/// updatable metadata, and lock metadata that governs whether the object can
/// be updated, transferred, or destroyed. The selected
/// [`NotarizationMethod`] determines which mutations are allowed after
/// creation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnChainNotarization {
    /// The unique identifier of the notarization.
    pub id: UID,
    /// Notarized data and its associated state metadata.
    ///
    /// The `state` of a notarization contains the notarized data and metadata
    /// associated with the current version of the `state`.
    ///
    /// Mutability depends on the Notarization Method:
    /// * `Dynamic`: updatable after creation via
    ///   [`NotarizationClient::update_state`](crate::client::NotarizationClient::update_state).
    /// * `Locked`: immutable after creation.
    pub state: State,
    /// Provenance fixed at creation time.
    ///
    /// Carries the creation timestamp, the optional immutable description,
    /// and the optional [`LockMetadata`](crate::core::types::LockMetadata).
    /// Provides immutable information, assertions, and guarantees for third
    /// parties and cannot be updated after creation.
    pub immutable_metadata: ImmutableMetadata,
    /// Free-form metadata providing context or additional information for
    /// third parties.
    ///
    /// Mutability depends on the Notarization Method:
    /// * `Dynamic`: updatable after creation via
    ///   [`NotarizationClient::update_metadata`](crate::client::NotarizationClient::update_metadata); updates do not
    ///   bump `state_version_count` nor change `last_state_change_at`.
    /// * `Locked`: immutable after creation.
    ///
    /// `updatable_metadata` can be updated independently of `state`.
    pub updatable_metadata: Option<String>,
    /// Timestamp of the most recent `state` change, in milliseconds since
    /// the Unix epoch.
    pub last_state_change_at: u64,
    /// Number of times `state` has been updated since creation. `0` means
    /// the state has not been updated since creation.
    pub state_version_count: u64,
    /// Notarization Method governing the mutation and destruction rules of
    /// this notarization.
    pub method: NotarizationMethod,
    /// The owner of the notarization.
    #[serde(skip, default = "iota_address_zero")]
    pub owner: Address,
}

fn iota_address_zero() -> Address {
    Address::ZERO
}

#[cfg(feature = "irl")]
pub mod irl_integration {
    use iota_caip::iota::{Address, IotaNetwork, IotaResourceLocator};
    use iota_caip::resource::RelativeUrl;
    use product_common::network_name::NetworkName;

    use super::OnChainNotarization;

    impl OnChainNotarization {
        /// Returns a builder for creating IOTA Resource Locators (IRLs) pointing within this notarization.
        /// # Example
        /// ```ignore
        /// let notarization: OnChainNotarization = ...;
        /// let notarized_data_irl = notarization
        ///     .iota_resource_locator_builder(notarization_client.network())
        ///     .data();
        ///
        /// assert_eq!(notarized_data_irl.to_string(), format!("iota:{}/{}/state/data", notarization_client.network().as_ref(), notarization.id.object_id()));
        /// ```
        pub fn iota_resource_locator_builder(&self, network: &NetworkName) -> NotarizationResourceBuilder {
            NotarizationResourceBuilder {
                network: IotaNetwork::custom(network.as_ref()).expect("valid network"),
                notarization_id: Address::new(self.id.id.bytes.into_bytes()),
            }
        }
    }

    /// A builder for creating IOTA Resource Locators (IRLs) pointing within a notarization.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NotarizationResourceBuilder {
        network: IotaNetwork,
        notarization_id: Address,
    }

    impl NotarizationResourceBuilder {
        /// Returns an IRL referencing this [OnChainNotarization] state's data.
        pub fn data(&self) -> IotaResourceLocator {
            self.make_irl("/state/data")
        }

        /// Returns an IRL referencing this [OnChainNotarization] state's metadata.
        pub fn state_metadata(&self) -> IotaResourceLocator {
            self.make_irl("/state/metadata")
        }

        /// Returns an IRL referencing this [OnChainNotarization]'s immutable metadata.
        pub fn immutable_metadata(&self) -> IotaResourceLocator {
            self.make_irl("/immutable_metadata")
        }

        /// Returns an IRL referencing this [OnChainNotarization]'s updatable metadata.
        pub fn updatable_metadata(&self) -> IotaResourceLocator {
            self.make_irl("/updatable_metadata")
        }

        /// Returns an IRL referencing this [OnChainNotarization]'s owner.
        pub fn owner(&self) -> IotaResourceLocator {
            self.make_irl("/owner")
        }

        fn make_irl(&self, path: &str) -> IotaResourceLocator {
            IotaResourceLocator::new(
                self.network.clone(),
                self.notarization_id,
                RelativeUrl::parse(path).expect("valid relative URL"),
            )
        }
    }
}
