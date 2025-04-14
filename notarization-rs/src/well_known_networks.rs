// Copyright 2020-2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use identity_iota_core::network::NetworkName;
use identity_iota_interaction::types::base_types::ObjectID;
use phf::{phf_map, Map};

/// A Mapping `network_id` -> metadata needed by the library.
pub(crate) static IOTA_NETWORKS: Map<&str, NotarizationNetworkMetadata> = phf_map! {
  "e678123a" => NotarizationNetworkMetadata::new(
    Some("devnet"),
    &["0x00"],
  ),
  "2304aa97" => NotarizationNetworkMetadata::new(
    Some("testnet"),
    &["0x00"],
  ),
};

/// `iota_notarization` package information for a given network.
#[derive(Debug)]
pub(crate) struct NotarizationNetworkMetadata {
    pub alias: Option<&'static str>,
    /// `package[0]` is the current version, `package[1]`
    /// is the version before, and so forth.
    pub package: &'static [&'static str],
}

/// Returns the [`NotarizationNetworkMetadata`] for a given network, if any.
pub(crate) fn network_metadata(network_id: &str) -> Option<&'static NotarizationNetworkMetadata> {
    IOTA_NETWORKS.get(network_id)
}

impl NotarizationNetworkMetadata {
    const fn new(alias: Option<&'static str>, pkgs: &'static [&'static str]) -> Self {
        assert!(!pkgs.is_empty());
        Self {
            alias,
            package: pkgs,
        }
    }

    /// Returns the latest `IotaNotarization` package ID on this network.
    pub(crate) fn latest_pkg_id(&self) -> ObjectID {
        self.package
            .first()
            .expect("a package was published")
            .parse()
            .expect("valid package ID")
    }

    /// Returns a [`NetworkName`] if `alias` is set.
    pub(crate) fn network_alias(&self) -> Option<NetworkName> {
        self.alias.map(|alias| {
            NetworkName::try_from(alias)
                .expect("an hardcoded network alias is valid (unless a dev messed it up)")
        })
    }
}

// #[cfg(test)]
// mod test {
//   use identity_iota_interaction::IotaClientBuilder;

//   use crate::notarization::NotarizationBuilder;
//   use crate::notarization::Notarization;

//   #[tokio::test]
//   async fn notarization_client_connection_to_devnet_works() -> anyhow::Result<()> {
//     let client = Notarization::new(NotarizationBuilder::default().build_devnet().await?).await?;
//     assert_eq!(client.network().as_ref(), "devnet");
//     Ok(())
//   }

//   #[tokio::test]
//   async fn notarization_client_connection_to_testnet_works() -> anyhow::Result<()> {
//     let client = Notarization::new(NotarizationBuilder::default().build_testnet().await?).await?;
//     assert_eq!(client.network().as_ref(), "testnet");
//     Ok(())
//   }
// }
