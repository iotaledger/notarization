// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Action codes for audit trail operations.
///
/// Each protected operation on the audit trail has a corresponding u16 action code.
/// These codes are used in `OperationCap` permissions to grant specific operations.
module audit_trail::actions;

// ===== Operational actions (require capability type + property validation) =====

const ADD_RECORD: u16 = 1;
const CORRECT_RECORD: u16 = 2;
const DELETE_RECORD: u16 = 3;
const DELETE_ALL_RECORDS: u16 = 4;
const UPDATE_METADATA: u16 = 5;
const DELETE_METADATA: u16 = 6;
const UPDATE_LOCKING_CONFIG: u16 = 7;
const UPDATE_LOCKING_CONFIG_FOR_DELETE_RECORD: u16 = 8;
const UPDATE_LOCKING_CONFIG_FOR_DELETE_TRAIL: u16 = 9;
const UPDATE_LOCKING_CONFIG_FOR_WRITE: u16 = 10;
const ADD_RECORD_TAGS: u16 = 11;
const DELETE_RECORD_TAGS: u16 = 12;

// ===== Governance actions (root authority / admin only, no property validation) =====

const DELETE_AUDIT_TRAIL: u16 = 100;
const MIGRATE: u16 = 101;

// ===== Public accessors =====

public fun add_record(): u16 { ADD_RECORD }
public fun correct_record(): u16 { CORRECT_RECORD }
public fun delete_record(): u16 { DELETE_RECORD }
public fun delete_all_records(): u16 { DELETE_ALL_RECORDS }
public fun update_metadata(): u16 { UPDATE_METADATA }
public fun delete_metadata(): u16 { DELETE_METADATA }
public fun update_locking_config(): u16 { UPDATE_LOCKING_CONFIG }
public fun update_locking_config_for_delete_record(): u16 { UPDATE_LOCKING_CONFIG_FOR_DELETE_RECORD }
public fun update_locking_config_for_delete_trail(): u16 { UPDATE_LOCKING_CONFIG_FOR_DELETE_TRAIL }
public fun update_locking_config_for_write(): u16 { UPDATE_LOCKING_CONFIG_FOR_WRITE }
public fun add_record_tags(): u16 { ADD_RECORD_TAGS }
public fun delete_record_tags(): u16 { DELETE_RECORD_TAGS }
public fun delete_audit_trail(): u16 { DELETE_AUDIT_TRAIL }
public fun migrate(): u16 { MIGRATE }
