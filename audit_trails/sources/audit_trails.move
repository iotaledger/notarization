// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Audit Trails - Tamper-proof sequential record chains with RBAC
module audit_trails_poc::audit_trails;

use iota::clock::Clock;
use iota::vec_map::VecMap;
use iota::vec_set::VecSet;
use std::string::String;

// ===== Core Structures =====

/// Controls when records can be deleted
public struct LockingConfig has copy, drop, store {
    time_window_seconds: Option<u64>,
    count_window: Option<u64>,
}

/// Immutable trail metadata (set at creation)
public struct TrailMetadata has store {
    name: Option<String>,
    description: Option<String>,
}

public struct Permission has copy, drop, store {}

/// Shared audit trail object
public struct AuditTrail has key, store {
    id: UID,
    locking_config: LockingConfig,
    permissions: VecMap<String, VecSet<Permission>>,
    immutable_metadata: TrailMetadata,
    updatable_metadata: Option<String>,
    issued_capabilities: VecSet<ID>,
    creator: address,
    created_at: u64,
    record_count: u64,
}

/// A single record in the audit trail
public struct Record<D: store + drop + copy> has key, store {
    id: UID,
    trail_id: ID,
    stored_data: D,
    record_metadata: Option<String>,
    previous_record_id: Option<ID>,
    sequence_number: u64,
    added_by: address,
    added_at: u64,
}
