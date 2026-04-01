// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::rpc_types::{IotaData as _, IotaObjectDataOptions};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::{IotaClientTrait, OptionalSync};
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::OnChainAuditTrail;
use crate::error::Error;

pub(crate) async fn get_audit_trail<C>(trail_id: ObjectID, client: &C) -> Result<OnChainAuditTrail, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let data = client
        .client_adapter()
        .read_api()
        .get_object_with_options(trail_id, IotaObjectDataOptions::bcs_lossless())
        .await
        .map_err(|e| Error::UnexpectedApiResponse(format!("failed to fetch trail {} object; {e}", trail_id)))?
        .data
        .ok_or_else(|| Error::UnexpectedApiResponse(format!("trail {} data not found", trail_id)))?;

    data.bcs
        .ok_or_else(|| Error::UnexpectedApiResponse(format!("trail {} missing bcs object content", trail_id)))?
        .try_into_move()
        .ok_or_else(|| Error::UnexpectedApiResponse(format!("trail {} bcs content is not a move object", trail_id)))?
        .deserialize()
        .map_err(|e| Error::UnexpectedApiResponse(format!("failed to decode trail {} bcs data; {e}", trail_id)))
}
