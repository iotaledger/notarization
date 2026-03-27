// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Record tag types and helper predicates for audit trails.
module audit_trail::record_tags;

use iota::{vec_map::{Self, VecMap}, vec_set::{Self, VecSet}};
use std::string::String;

// ----------- RoleTags -------

/// Stores all record tag related data associated with a role.
/// Contains a list of allowlisted tags for the role.
public struct RoleTags has copy, drop, store {
    tags: VecSet<String>,
}

/// Create a new `RoleTags`.
public fun new_role_tags(tags: vector<String>): RoleTags {
    RoleTags {
        tags: vec_set::from_keys(tags),
    }
}

/// Get the allowlisted record tags for a role from a `RoleTags`.
public fun tags(self: &RoleTags): &VecSet<String> {
    &self.tags
}

// ----------- TagRegistry -------

/// A registry of tags available for use on an audit trail, along with usage counts
/// to track how many records and roles are currently using each tag.
/// Usage counts for roles and tags are summed and build a combined usage count.
public struct TagRegistry has copy, drop, store {
    tag_map: VecMap<String, u64>,
}

/// Get a mapping of record tag names to `u64`.
public fun tag_map(self: &TagRegistry): &VecMap<String, u64> {
    &self.tag_map
}

/// Create a `TagRegistry` with zeroed usage counts to manage a list of available tags to be
/// associated with records and roles on an audit trail.
public(package) fun new_tag_registry(mut tags: vector<String>): TagRegistry {
    let mut usage = vec_map::empty<String, u64>();
    tags.reverse();

    while (tags.length() != 0) {
        vec_map::insert(&mut usage, tags.pop_back(), 0);
    };

    TagRegistry { tag_map: usage }
}

/// Destroys the `TagRegistry` by emptying the internal tag map and then destroying it.
public(package) fun destroy(mut self: TagRegistry) {
    while (!self.tag_map.is_empty()) {
        let (_, _) = self.tag_map.pop();
    };
    self.tag_map.destroy_empty();
}

public(package) fun insert_tag(self: &mut TagRegistry, tag: String, usage_count: u64) {
    self.tag_map.insert(tag, usage_count);
}

public(package) fun remove_tag(self: &mut TagRegistry, tag: &String) {
    self.tag_map.remove(tag);
}

public(package) fun tag_keys(self: &TagRegistry): vector<String> {
    iota::vec_map::keys(&self.tag_map)
}

/// Returns true when all provided `role_tags` (tags associated with a role) are contained in the `TagRegistry`.
public(package) fun contains_all_role_tags(self: &TagRegistry, role_tags: &Option<RoleTags>): bool {
    if (!role_tags.is_some()) {
        return true
    };

    let tags = &option::borrow(role_tags).tags;
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

/// Returns the current combined usage count (sum of role and record usages) for a tag.
/// Returns `Option::none()` if the tag is not contained in the registry.
public(package) fun usage_count(self: &TagRegistry, tag: &String): Option<u64> {
    if (self.tag_map.contains(tag)) {
        option::some(*self.tag_map.get(tag))
    } else {
        option::none()
    }
}

/// Increments the usage count for a tag by 1.
/// Will be without effect if the tag is not contained in the registry.
public(package) fun increment_usage_count(self: &mut TagRegistry, tag: &String) {
    if (self.tag_map.contains(tag)) {
        let counters = vec_map::get_mut(&mut self.tag_map, tag);
        *counters = *counters + 1;
    };
}

/// Decrements the usage count for a tag by 1.
/// Will be without effect if the tag is not contained in the registry.
public(package) fun decrement_usage_count(self: &mut TagRegistry, tag: &String) {
    if (self.tag_map.contains(tag)) {
        let counters = vec_map::get_mut(&mut self.tag_map, tag);
        *counters = *counters - 1;
    };
}

/// Returns if the specified is in use.
/// Returns false if the tag is not contained in the registry.
public(package) fun is_in_use(self: &TagRegistry, tag: &String): bool {
    (*self.usage_count(tag).borrow_with_default(&0)) > 0
}
