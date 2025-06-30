// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Client implementations for interacting with notarizations on the IOTA blockchain.
//!
//! This module provides two client types:
//! - [`read_only`]: Read-only access to notarization data
//! - [`full_client`]: Full read-write access with transaction capabilities

use iota_interaction::IotaClientTrait;
use product_common::network_name::NetworkName;

use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;

pub mod full_client;
pub mod read_only;

pub use full_client::*;
pub use read_only::*;

/// Returns the network-id also known as chain-identifier provided by the specified iota_client
async fn network_id(iota_client: &IotaClientAdapter) -> Result<NetworkName, Error> {
    let network_id = iota_client
        .read_api()
        .get_chain_identifier()
        .await
        .map_err(|e| Error::RpcError(e.to_string()))?;
    Ok(network_id.try_into().expect("chain ID is a valid network name"))
}
