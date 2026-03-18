// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Record tag types and helper predicates for audit trails.
module audit_trail::record_tags;

use audit_trail::permission::Permission;
use iota::{vec_map::{Self, VecMap}, vec_set::{Self, VecSet}};
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

/// Create a zeroed usage counter for all tags in the trail list
public(package) fun new_usage(mut tags: vector<String>): VecMap<String, u64> {
    let mut usage = vec_map::empty<String, u64>();
    tags.reverse();

    while (tags.length() != 0) {
        vec_map::insert(&mut usage, tags.pop_back(), 0);
    };

    usage
}

/// Returns true when all provided role tags are defined on the trail.
public(package) fun defined_for_trail(
    available_tags: &VecMap<String, u64>,
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
        if (!iota::vec_map::contains(available_tags, &allowed_tag_keys[i])) {
            return false
        };
        i = i + 1;
    };

    true
}

/// Returns true when the requested tag exists in the trail registry.
public(package) fun is_defined(available_tags: &VecMap<String, u64>, tag: &String): bool {
    iota::vec_map::contains(available_tags, tag)
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

/// Returns the current combined usage count for a tag across records and roles.
public(package) fun usage_count(usage: &VecMap<String, u64>, tag: &String): u64 {
    if (vec_map::contains(usage, tag)) {
        *vec_map::get(usage, tag)
    } else {
        0
    }
}

public(package) fun increment_tag_usage(usage: &mut VecMap<String, u64>, tag: &String) {
    let count = vec_map::get_mut(usage, tag);
    *count = *count + 1;
}

public(package) fun decrement_tag_usage(usage: &mut VecMap<String, u64>, tag: &String) {
    let count = vec_map::get_mut(usage, tag);
    *count = *count - 1;
}
