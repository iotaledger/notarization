// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Record tag types and helper predicates for audit trails.
module audit_trail::record_tags;

use audit_trail::{permission::Permission, record::{Self, Record}};
use iota::{linked_table::{Self, LinkedTable}, vec_set::{Self, VecSet}};
use std::string::String;
use tf_components::{capability::Capability, role_map::{Self, RoleMap}};

/// Stores all record tag related data associated with a role in the RoleMap.
public struct RecordTags has copy, drop, store {
    tags: VecSet<String>,
}

/// Create a role-scoped record-tag access list.
public fun new_record_tags(tags: vector<String>): RecordTags {
    RecordTags {
        tags: vec_set::from_keys(tags),
    }
}

/// Get the allowlisted record tags for a role.
public fun allowed_record_tags(record_tags: &RecordTags): &VecSet<String> {
    &record_tags.tags
}

/// Returns true when all provided role tags are defined on the trail.
public(package) fun defined_for_trail(
    available_tags: &VecSet<String>,
    record_tags: &Option<RecordTags>,
): bool {
    if (!record_tags.is_some()) {
        return true
    };

    let tags = &option::borrow(record_tags).tags;
    let allowed_tag_keys = iota::vec_set::keys(tags);
    let mut i = 0;
    let tag_count = allowed_tag_keys.length();

    while (i < tag_count) {
        if (!iota::vec_set::contains(available_tags, &allowed_tag_keys[i])) {
            return false
        };
        i = i + 1;
    };

    true
}

/// Returns true when the requested tag exists in the trail registry.
public(package) fun is_defined(available_tags: &VecSet<String>, tag: &String): bool {
    iota::vec_set::contains(available_tags, tag)
}

/// Returns true when the capability's role data allows the requested tag.
public(package) fun role_allows(
    roles: &RoleMap<Permission, RecordTags>,
    cap: &Capability,
    tag: &String,
): bool {
    let role_tags = role_map::get_role_data(roles, cap.role());
    if (!role_tags.is_some()) {
        return false
    };

    let tags = &option::borrow(role_tags).tags;
    iota::vec_set::contains(tags, tag)
}

/// Returns true when any live record currently uses the provided tag.
public(package) fun is_in_use<D: store + copy>(
    records: &LinkedTable<u64, Record<D>>,
    sequence_number: u64,
    tag: &String,
): bool {
    let mut current_sequence = 0;
    while (current_sequence < sequence_number) {
        if (linked_table::contains(records, current_sequence)) {
            let stored_record = linked_table::borrow(records, current_sequence);
            let record_tag = record::tag(stored_record);

            if (record_tag.is_some() && option::borrow(record_tag) == tag) {
                return true
            };
        };

        current_sequence = current_sequence + 1;
    };

    false
}
