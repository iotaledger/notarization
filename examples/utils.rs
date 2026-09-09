// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[path = "notarization/utils.rs"]
mod notarization;
#[path = "poi/utils.rs"]
mod poi;

pub use notarization::{
    get_funded_audit_trail_client, get_funded_notarization_client, get_notarization_read_only_client,
    issue_tagged_record_role,
};
pub use poi::{NotarizationTargets, PoiContext, prepare_poi_example};
