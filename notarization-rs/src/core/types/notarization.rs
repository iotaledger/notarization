// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::IotaAddress;
use iota_interaction::types::id::UID;
use serde::{Deserialize, Serialize};

use super::NotarizationMethod;
use super::metadata::ImmutableMetadata;
use super::state::State;

/// A notarization record stored on the blockchain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnChainNotarization {
    /// The unique identifier of the notarization.
    pub id: UID,
    /// The state of the notarization.
    ///
    /// The `state` of a notarization contains the notarized data and metadata associated with
    /// the current version of the `state`.
    ///
    /// `state` can be updated depending on the used `NotarizationMethod`:
    /// - Dynamic: Can be updated anytime after notarization creation
    /// - Locked: Immutable after notarization creation
    ///
    /// Use `NotarizationClient::update_state()` for `state` updates.
    pub state: State,
    /// The immutable metadata of the notarization.
    ///
    /// NOTE:
    /// - provides immutable information, assertions and guaranties for third parties
    /// - `immutable_metadata` are automatically created at creation time and cannot be updated thereafter
    pub immutable_metadata: ImmutableMetadata,
    /// The updatable metadata of the notarization.
    ///
    /// Provides context or additional information for third parties
    ///
    /// `updatable_metadata` can be updated depending on the used `NotarizationMethod`:
    /// - Dynamic: Can be updated anytime after notarization creation
    /// - Locked: Immutable after notarization creation
    ///
    /// NOTE:
    /// - `updatable_metadata` can be updated independently of `state`
    /// - Updating `updatable_metadata` does not increase the `state_version_count`
    /// - Updating `updatable_metadata` does not change the `last_state_change_at` timestamp
    /// - Use `NotarizationClient::update_metadata()` for `updatable_metadata` updates.
    pub updatable_metadata: Option<String>,
    /// The timestamp of the last state change (milliseconds since UNIX epoch)
    pub last_state_change_at: u64,
    /// The number of state changes.
    pub state_version_count: u64,
    /// The method of the notarization.
    pub method: NotarizationMethod,
    /// The owner of the notarization.
    #[serde(skip)]
    pub owner: IotaAddress,
}

#[cfg(feature = "irl")]
pub mod irl_integration {
    use iota_caip::iota::{IotaAddress, IotaNetwork, IotaResourceLocator};
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
                notarization_id: IotaAddress::new(self.id.id.bytes.into_bytes()),
            }
        }
    }

    /// A builder for creating IOTA Resource Locators (IRLs) pointing within a notarization.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NotarizationResourceBuilder {
        network: IotaNetwork,
        notarization_id: IotaAddress,
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
