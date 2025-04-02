// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use identity_iota_core::iota_interaction_adapter::IotaClientAdapter;
use identity_iota_core::network::NetworkName;
use crate::error::Error;

use identity_iota_interaction::IotaClientTrait;

/// Returns the network-id also known as chain-identifier provided by the specified iota_client
pub async fn network_id(iota_client: &IotaClientAdapter) -> Result<NetworkName, Error> {
  let network_id = iota_client
    .read_api()
    .get_chain_identifier()
    .await
    .map_err(|e| Error::RpcError(e.to_string()))?;
  Ok(network_id.try_into().expect("chain ID is a valid network name"))
}