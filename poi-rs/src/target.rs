// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::Committee;
use iota_types::{
    base_types::ObjectRef,
    event::{Event, EventID},
    object::Object,
};
use serde::{Deserialize, Serialize};

/// Target claims authenticated by a Proof of Inclusion.
///
/// Object and event targets are authenticated through the transaction evidence in
/// the proof. Committee targets authenticate the next epoch committee recorded in
/// an end-of-epoch checkpoint summary.
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
    /// Creates an empty target set.
    ///
    /// Empty targets are mainly useful while constructing proofs incrementally.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an object target by object reference and object contents.
    ///
    /// Verification checks that the object computes to the supplied reference and
    /// that the transaction effects include the reference.
    pub fn add_object(mut self, object_ref: ObjectRef, object: Object) -> Self {
        self.objects.push((object_ref, object));
        self
    }

    /// Adds an event target by event ID and event contents.
    ///
    /// Verification checks that the event belongs to the transaction and matches
    /// the event stored at the requested event sequence.
    pub fn add_event(mut self, event_id: EventID, event: Event) -> Self {
        self.events.push((event_id, event));
        self
    }

    /// Adds a next-epoch committee target.
    ///
    /// Verification checks that the checkpoint is an end-of-epoch checkpoint and
    /// that its next committee matches the supplied committee.
    pub fn set_committee(mut self, committee: Committee) -> Self {
        self.committee = Some(committee);
        self
    }
}
