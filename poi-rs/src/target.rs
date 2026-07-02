// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk_types::{CheckpointContents, Event, Transaction, TransactionEffects, TransactionEvents};
use iota_types::committee::Committee;
use iota_types::{base_types::ObjectRef, digests::TransactionDigest, event::EventID, object::Object};
use serde::{Deserialize, Serialize};

/// Define aspects of IOTA state that need to be certified in a proof
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct ProofTargets {
    /// Objects that need to be certified.
    pub objects: Vec<(ObjectRef, Object)>,

    /// Events that need to be certified.
    pub events: Vec<(EventID, Event)>,

    /// The next committee being certified.
    pub committee: Option<Committee>,
}

impl ProofTargets {
    /// Create a new empty proof target. An empty proof target still ensures
    /// that the checkpoint summary is correct.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an object to be certified by object reference and content. A
    /// verified proof will ensure that both the reference and content are
    /// correct. Note that some content is metadata such as the transaction
    /// that created this object.
    pub fn add_object(mut self, object_ref: ObjectRef, object: Object) -> Self {
        self.objects.push((object_ref, object));
        self
    }

    /// Add an event to be certified by event ID and content. A verified proof
    /// will ensure that both the ID and content are correct.
    pub fn add_event(mut self, event_id: EventID, event: Event) -> Self {
        self.events.push((event_id, event));
        self
    }

    /// Add the next committee to be certified. A verified proof will ensure
    /// that the next committee is correct.
    pub fn set_committee(mut self, committee: Committee) -> Self {
        self.committee = Some(committee);
        self
    }
}
