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

/// Builder that assembles the parameters for creating a new audit trail.
///
/// @remarks
/// The resulting transaction publishes the trail as a *shared* object, seeds the reserved
/// {@link RoleMap.initialAdminRoleName | Admin} role with the recommended admin permissions, and
/// transfers a freshly minted initial-admin {@link Capability} to the configured admin address. An
/// admin address must be set (either through {@link AuditTrailBuilder.withAdmin} or by constructing
/// the builder via {@link AuditTrailClient.createTrail}, which seeds it with the signer); otherwise
/// {@link AuditTrailBuilder.finish} produces a transaction that fails to build. When an initial
/// record is set, its tag — if any — must already be in the configured record-tag list.
#[wasm_bindgen(js_name = AuditTrailBuilder, inspectable)]
pub struct WasmAuditTrailBuilder(pub(crate) AuditTrailBuilder);

#[wasm_bindgen(js_class = AuditTrailBuilder)]
impl WasmAuditTrailBuilder {
    /// Sets the initial record using a UTF-8 string payload.
    ///
    /// @remarks
    /// The record is stored at sequence number `0`.
    ///
    /// When `tag` is provided it must already appear in the list passed to
    /// {@link AuditTrailBuilder.withRecordTags}; the on-chain call aborts otherwise.
    /// Bumps the tag's usage count on success.
    ///
    /// @param data - UTF-8 text payload for the initial record.
    /// @param metadata - Optional application-defined metadata stored alongside the record.
    /// @param tag - Optional trail-owned tag attached to the record.
    ///
    /// @returns The same builder, with the initial record configured.
    #[wasm_bindgen(js_name = withInitialRecordString)]
    pub fn with_initial_record_string(self, data: String, metadata: Option<String>, tag: Option<String>) -> Self {
        Self(self.0.with_initial_record_parts(data, metadata, tag))
    }

    /// Sets the initial record using a raw byte payload.
    ///
    /// @remarks
    /// The record is stored at sequence number `0`.
    /// When `tag` is provided it must already appear in the list passed to
    /// {@link AuditTrailBuilder.withRecordTags}; the on-chain call aborts otherwise.
    /// Bumps the tag's usage count on success.
    ///
    /// @param data - Raw bytes stored as the initial record payload.
    /// @param metadata - Optional application-defined metadata stored alongside the record.
    /// @param tag - Optional trail-owned tag attached to the record.
    ///
    /// @returns The same builder, with the initial record configured.
    #[wasm_bindgen(js_name = withInitialRecordBytes)]
    pub fn with_initial_record_bytes(
        self,
        data: js_sys::Uint8Array,
        metadata: Option<String>,
        tag: Option<String>,
    ) -> Self {
        Self(self.0.with_initial_record_parts(data.to_vec(), metadata, tag))
    }

    /// Sets the trail's {@link ImmutableMetadata} (name and optional description).
    ///
    /// @remarks
    /// Stored once at trail creation and exposed read-only thereafter. Use
    /// {@link AuditTrailBuilder.withUpdatableMetadata} for the mutable counterpart.
    ///
    /// @param name - Human-readable trail name.
    /// @param description - Optional human-readable description.
    ///
    /// @returns The same builder, with the immutable metadata configured.
    #[wasm_bindgen(js_name = withTrailMetadata)]
    pub fn with_trail_metadata(self, name: String, description: Option<String>) -> Self {
        Self(self.0.with_trail_metadata_parts(name, description))
    }

    /// Sets the trail's `updatableMetadata` field.
    ///
    /// @remarks
    /// This field can later be replaced or cleared by holders of {@link Permission.UpdateMetadata}
    /// via {@link AuditTrailHandle.updateMetadata}.
    ///
    /// @param metadata - Initial value of the trail's `updatableMetadata` field.
    ///
    /// @returns The same builder, with the updatable metadata configured.
    #[wasm_bindgen(js_name = withUpdatableMetadata)]
    pub fn with_updatable_metadata(self, metadata: String) -> Self {
        Self(self.0.with_updatable_metadata(metadata))
    }

    /// Sets the {@link LockingConfig} for the trail.
    ///
    /// @remarks
    /// `config.deleteTrailLock` must not be {@link TimeLock.withUntilDestroyed}; trail creation
    /// aborts on-chain otherwise.
    ///
    /// @param config - Combined delete-record window, delete-trail lock, and write lock.
    ///
    /// @returns The same builder, with the locking configuration applied.
    #[wasm_bindgen(js_name = withLockingConfig)]
    pub fn with_locking_config(self, config: WasmLockingConfig) -> Self {
        Self(self.0.with_locking_config(config.into()))
    }

    /// Sets the canonical list of record tags owned by the trail.
    ///
    /// @remarks
    /// Every tag name later referenced by an initial record, an {@link TrailRecords.add} call, or a
    /// role's {@link RoleTags} allowlist must appear in this list. Tags are inserted with a usage
    /// count of zero.
    ///
    /// @param tags - Tag names that the trail will recognize.
    ///
    /// @returns The same builder, with the record-tag registry configured.
    #[wasm_bindgen(js_name = withRecordTags)]
    pub fn with_record_tags(self, tags: Vec<String>) -> Self {
        Self(self.0.with_record_tags(tags))
    }

    /// Sets the initial admin address.
    ///
    /// @remarks
    /// On execution the trail's role map is seeded with a single role named `"Admin"` carrying the
    /// recommended admin permissions, and a freshly minted initial-admin capability is transferred
    /// to this address. Setting an admin is required before {@link AuditTrailBuilder.finish} can
    /// produce a viable transaction; constructing the builder via
    /// {@link AuditTrailClient.createTrail} already seeds it with the signer address.
    ///
    /// @param admin - Address that will receive the initial-admin capability.
    ///
    /// @returns The same builder, with the admin address configured.
    ///
    /// @throws When `admin` is not a valid IOTA address.
    #[wasm_bindgen(js_name = withAdmin)]
    pub fn with_admin(self, admin: WasmIotaAddress) -> Result<Self> {
        let admin = parse_wasm_iota_address(&admin)?;
        Ok(Self(self.0.with_admin(admin)))
    }

    /// Finalizes the builder into a transaction wrapper.
    ///
    /// @remarks
    /// On execution the audit-trail package shares the new trail object, seeds the reserved
    /// {@link RoleMap.initialAdminRoleName | Admin} role, transfers an initial-admin capability to
    /// the configured admin address, and optionally stores the initial record at sequence number
    /// `0`.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link CreateTrail} transaction.
    ///
    /// @throws When the builder is missing a required field or its initial record references a tag
    /// that is not in the record-tag list.
    ///
    /// Emits an {@link AuditTrailCreated} event on success.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<CreateTrail>")]
    pub fn finish(self) -> Result<WasmTransactionBuilder> {
        Ok(into_transaction_builder(WasmCreateTrail::new(self)))
    }
}
