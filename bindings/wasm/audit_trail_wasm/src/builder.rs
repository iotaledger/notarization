// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trail::core::builder::AuditTrailBuilder;
use iota_interaction_ts::wasm_error::Result;
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::{into_transaction_builder, parse_wasm_iota_address};
use product_common::bindings::WasmIotaAddress;
use wasm_bindgen::prelude::*;

use crate::trail::WasmCreateTrail;
use crate::types::WasmLockingConfig;

/// Trail-creation builder exposed to wasm consumers.
///
/// The resulting create transaction publishes the trail as a *shared* object, seeds a reserved
/// `Admin` role with the recommended admin permissions, and transfers a freshly minted initial-admin
/// capability to the configured admin address. The admin address must be set (either via
/// `withAdmin` or by constructing the builder from `AuditTrailClient.createTrail()` which seeds it
/// from the signer); otherwise `finish()` produces a transaction that fails to build. When an
/// initial record is set, its tag (if any) must already be in the configured record-tag list.
#[wasm_bindgen(js_name = AuditTrailBuilder, inspectable)]
pub struct WasmAuditTrailBuilder(pub(crate) AuditTrailBuilder);

#[wasm_bindgen(js_class = AuditTrailBuilder)]
impl WasmAuditTrailBuilder {
    /// Sets the initial record using a UTF-8 string payload.
    ///
    /// The record will be stored at sequence number `0`. When `tag` is set it must already be
    /// present in the list passed to [`withRecordTags`](Self::with_record_tags); the on-chain call
    /// aborts otherwise and bumps the tag's usage count on success.
    #[wasm_bindgen(js_name = withInitialRecordString)]
    pub fn with_initial_record_string(self, data: String, metadata: Option<String>, tag: Option<String>) -> Self {
        Self(self.0.with_initial_record_parts(data, metadata, tag))
    }

    /// Sets the initial record using raw bytes.
    ///
    /// The record will be stored at sequence number `0`. When `tag` is set it must already be
    /// present in the list passed to [`withRecordTags`](Self::with_record_tags); the on-chain call
    /// aborts otherwise and bumps the tag's usage count on success.
    #[wasm_bindgen(js_name = withInitialRecordBytes)]
    pub fn with_initial_record_bytes(
        self,
        data: js_sys::Uint8Array,
        metadata: Option<String>,
        tag: Option<String>,
    ) -> Self {
        Self(self.0.with_initial_record_parts(data.to_vec(), metadata, tag))
    }

    /// Sets the trail's `ImmutableMetadata` (name and optional description).
    ///
    /// Stored once at trail creation and exposed read-only thereafter. Use
    /// [`withUpdatableMetadata`](Self::with_updatable_metadata) for the mutable counterpart.
    #[wasm_bindgen(js_name = withTrailMetadata)]
    pub fn with_trail_metadata(self, name: String, description: Option<String>) -> Self {
        Self(self.0.with_trail_metadata_parts(name, description))
    }

    /// Sets the trail's `updatableMetadata` field.
    ///
    /// This field can later be replaced or cleared by holders of the `UpdateMetadata` permission
    /// via [`AuditTrailHandle.updateMetadata`](crate::trail_handle::WasmAuditTrailHandle::update_metadata).
    #[wasm_bindgen(js_name = withUpdatableMetadata)]
    pub fn with_updatable_metadata(self, metadata: String) -> Self {
        Self(self.0.with_updatable_metadata(metadata))
    }

    /// Sets the locking configuration for the trail.
    ///
    /// The `deleteTrailLock` of `config` must not be `TimeLock.withUntilDestroyed()`; trail
    /// creation aborts on-chain otherwise.
    #[wasm_bindgen(js_name = withLockingConfig)]
    pub fn with_locking_config(self, config: WasmLockingConfig) -> Self {
        Self(self.0.with_locking_config(config.into()))
    }

    /// Sets the canonical list of record tags owned by the trail.
    ///
    /// Every tag name later referenced by an initial record, an `addRecord` call, or a role's
    /// `roleTags` allowlist must appear in this list. Tags are inserted with a usage count of
    /// zero.
    #[wasm_bindgen(js_name = withRecordTags)]
    pub fn with_record_tags(self, tags: Vec<String>) -> Self {
        Self(self.0.with_record_tags(tags))
    }

    /// Sets the initial admin address.
    ///
    /// On execution the trail's role map is seeded with a single role named `"Admin"` carrying
    /// the recommended admin permissions, and a freshly minted initial-admin capability is
    /// transferred to this address. Setting an admin is required before [`finish`](Self::finish)
    /// can produce a viable transaction; constructing the builder via
    /// `AuditTrailClient.createTrail()` already seeds it with the signer address.
    #[wasm_bindgen(js_name = withAdmin)]
    pub fn with_admin(self, admin: WasmIotaAddress) -> Result<Self> {
        let admin = parse_wasm_iota_address(&admin)?;
        Ok(Self(self.0.with_admin(admin)))
    }

    /// Finalizes the builder into a transaction wrapper.
    ///
    /// The resulting transaction publishes the trail as a *shared* object, seeds the reserved
    /// `Admin` role, transfers an initial-admin capability to the configured admin address,
    /// optionally stores the initial record at sequence number `0`, and emits an
    /// `AuditTrailCreated` event on success.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<CreateTrail>")]
    pub fn finish(self) -> Result<WasmTransactionBuilder> {
        Ok(into_transaction_builder(WasmCreateTrail::new(self)))
    }
}
