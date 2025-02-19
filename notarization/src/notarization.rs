use identity_iota_core::NetworkName;

// use identity_iota_core::iota_interaction_adapter::IotaClientAdapter
// use identity_iota_core::iota_interaction_adapter::ObjectID
// ---------------------------------------------------------------
// use crate::iota_interaction_adapter::IotaClientAdapter;
// use crate::iota_interaction_adapter::ObjectID;

use iota_interaction_ts::IotaClientAdapter;
use identity_iota_interaction::types::base_types::ObjectID;

#[derive(Clone)]
pub struct Notarization {
  iota_client: IotaClientAdapter,
  iota_notarization_pkg_id: ObjectID,
  network: NetworkName,
}

impl Notarization {

}