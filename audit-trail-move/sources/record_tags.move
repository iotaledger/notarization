// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Record tag types and helper predicates for audit trails.
module audit_trail::record_tags;

use audit_trail::permission::Permission;
use iota::{vec_map::{Self, VecMap}, vec_set::{Self, VecSet}};
use std::string::String;
use tf_components::{capability::Capability, role_map::{Self, RoleMap}};

// ----------- RoleTagList -------

/// Stores all record tag related data associated with a role in the RoleMap.
/// Contains a list of allowlisted tags for the role.
public struct RoleTagList has copy, drop, store {
    tags: VecSet<String>,
}

/// Create a new `RoleTagList`.
public fun new_role_tag_list(tags: vector<String>): RoleTagList {
    RoleTagList {
        tags: vec_set::from_keys(tags),
    }
}

/// Get the allowlisted record tags for a role from a `RoleTagList`.
public fun tags(self: &RoleTagList): &VecSet<String> {
    &self.tags
}

// ----------- UsageCounters -------

/// Usage counters for a record tag, tracking how many records and roles are currently using the tag.
public struct UsageCounters has copy, drop, store {
    record: u64,
    role: u64,
}

/// Create a `UsageCounters` struct with the provided counts for record and role usage.
public fun new_usage_counters(record: u64, role: u64): UsageCounters {
    UsageCounters {
        record,
        role,
    }
}

/// Increment the record usage counter by 1.
public fun increment_record_usage_counter(self: &mut UsageCounters) {
    self.record = self.record + 1;
}

/// Increment the role usage counter by 1.
public fun increment_role_usage_counter(self: &mut UsageCounters) {
    self.role = self.role + 1;
}

/// Getter for the record usage counter.
public fun record_usage_counter(self: &UsageCounters): u64 {
    self.record
}

/// Getter for the role usage counter.
public fun role_usage_counter(self: &UsageCounters): u64 {
    self.role
}

// ----------- TagRegistry -------

/// A registry of tags available for use on an audit trail, along with `UsageCounters`
/// to track how many records and roles are currently using each tag.
public struct TagRegistry has copy, drop, store {
    tag_map: VecMap<String, UsageCounters>,
}

/// Get a mapping of record tag names to `UsageCounters`.
public fun tag_map(self: &TagRegistry): &VecMap<String, UsageCounters> {
    &self.tag_map
}

/// Create a `TagRegistry` with zeroed `UsageCounters` to manage a list of available tags to be
/// associated with records and roles on an audit trail.
public(package) fun new_tag_registry(mut tags: vector<String>): TagRegistry {
    let mut usage = vec_map::empty<String, UsageCounters>();
    tags.reverse();

    while (tags.length() != 0) {
        vec_map::insert(&mut usage, tags.pop_back(), new_usage_counters(0, 0));
    };

    TagRegistry { tag_map: usage }
}

/// Returns true when all provided `record_tags` (tags associated with a role) are contained in the `TagRegistry`.
public(package) fun defined_for_trail(
    self: &TagRegistry,
    record_tags: &Option<RoleTagList>,
): bool {
    if (!record_tags.is_some()) {
        return true
    };

    let tags = &option::borrow(record_tags).tags;
    let allowed_tag_keys = iota::vec_set::keys(tags);
    let mut i = 0;
    let tag_count = allowed_tag_keys.length();

    while (i < tag_count) {
        if (!iota::vec_map::contains(&self.tag_map, &allowed_tag_keys[i])) {
            return false
        };
        i = i + 1;
    };

    true
}

/// Returns true when the specified tag is contained in the `TagRegistry`.
public(package) fun contains(self: &TagRegistry, tag: &String): bool {
    iota::vec_map::contains(&self.tag_map, tag)
}

/// Returns the current `UsageCounters` for a tag.
/// Returns `Option::none()` if the tag is not contained in the registry.
public(package) fun usage_counters(self: &TagRegistry, tag: &String): Option<UsageCounters> {
    if (self.tag_map.contains(tag)) {
        option::some(*self.tag_map.get(tag))
    } else {
        option::none()
    }
}

/// Returns the current combined (summed) usage counts for a tag across records and roles.
/// Returns 0 if the tag is not contained in the registry.
public(package) fun usage_count_total(self: &TagRegistry, tag: &String): u64 {
    if (vec_map::contains(&self.tag_map, tag)) {
        let counters = vec_map::get(&self.tag_map, tag);
        counters.record + counters.role
    } else {
        0
    }
}

public(package) fun increment_tag_usage_for_records(self: &mut TagRegistry, tag: &String) {
    let counters = vec_map::get_mut(&mut self.tag_map, tag);
    counters.increment_record_usage_counter();
}

public(package) fun decrement_tag_usage_for_roles(self: &mut TagRegistry, tag: &String) {
    let counters = vec_map::get_mut(&mut self.tag_map, tag);
    counters.increment_role_usage_counter();
}


// ----------- RoleMap related -------

/// Returns true when the capability's role data allows the requested tag.
public(package) fun role_allows(
    roles: &RoleMap<Permission, RoleTagList>,
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