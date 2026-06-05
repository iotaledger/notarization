// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Client implementations for interacting with audit trails on the IOTA ledger.
//!
//! [`AuditTrailClientReadOnly`] is the entry point for read-only inspection and typed trail handles.
//! [`AuditTrailClient`] wraps a read-only client together with a signer so it can build write
//! transactions through the shared transaction infrastructure.

use iota_interaction::IotaClientTrait;
use product_common::network_name::NetworkName;

use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;

/// A signing client that can create audit-trail transaction builders.
pub mod full_client;
/// A read-only client that resolves package IDs and executes inspected calls.
pub mod read_only;

pub use full_client::*;
pub use read_only::*;

/// Resolves the network name reported by the given IOTA client.
async fn network_id(iota_client: &IotaClientAdapter) -> Result<NetworkName, Error> {
    let network_id = iota_client
        .read_api()
        .get_chain_identifier()
        .await
        .map_err(|e| Error::RpcError(e.to_string()))?;
    Ok(network_id.try_into().expect("chain ID is a valid network name"))
}
