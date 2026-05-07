// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Record tag types and helper predicates for audit trails.
module audit_trail::record_tags;

use audit_trail::permission::Permission;
use iota::{vec_map::{Self, VecMap}, vec_set::{Self, VecSet}};
use std::string::String;
use tf_components::{capability::Capability, role_map::{Self, RoleMap}};

// ----------- RoleTags -------

/// Stores all record tag related data associated with a role in the RoleMap.
/// Contains a list of allowlisted tags for the role.
public struct RoleTags has copy, drop, store {
    tags: VecSet<String>,
}

/// Creates a new `RoleTags` allowlisting the given record tags.
///
/// Returns the constructed `RoleTags`.
public fun new_role_tags(tags: vector<String>): RoleTags {
    RoleTags {
        tags: vec_set::from_keys(tags),
    }
}

/// Returns a reference to the set of record tags allowlisted by this `RoleTags`.
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

/// Returns a reference to the registry's mapping of tag names to combined usage counts.
public fun tag_map(self: &TagRegistry): &VecMap<String, u64> {
    &self.tag_map
}

/// Creates a `TagRegistry` listing the given tags with zero usage counts.
///
/// Returns the constructed `TagRegistry`.
public(package) fun new_tag_registry(mut tags: vector<String>): TagRegistry {
    let mut usage = vec_map::empty<String, u64>();
    tags.reverse();

    while (tags.length() != 0) {
        vec_map::insert(&mut usage, tags.pop_back(), 0);
    };

    TagRegistry { tag_map: usage }
}

/// Destroys the `TagRegistry`.
///
/// Empties the internal tag map and then destroys the empty container.
public(package) fun destroy(mut self: TagRegistry) {
    while (!self.tag_map.is_empty()) {
        let (_, _) = self.tag_map.pop();
    };
    self.tag_map.destroy_empty();
}

/// Inserts `tag` into the registry with the given initial `usage_count`.
public(package) fun insert_tag(self: &mut TagRegistry, tag: String, usage_count: u64) {
    self.tag_map.insert(tag, usage_count);
}

/// Removes `tag` from the registry.
public(package) fun remove_tag(self: &mut TagRegistry, tag: &String) {
    self.tag_map.remove(tag);
}

/// Returns the set of tag names currently registered, as a `vector<String>`.
public(package) fun tag_keys(self: &TagRegistry): vector<String> {
    iota::vec_map::keys(&self.tag_map)
}

/// Checks whether every tag listed in `role_tags` is registered.
///
/// `option::none()` is treated as the empty set and trivially satisfies the check.
///
/// Returns `true` when every tag in `role_tags` is contained in the registry, or
/// when `role_tags` is `option::none()`.
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

/// Checks whether `tag` is registered in the `TagRegistry`.
///
/// Returns `true` when `tag` is present.
public(package) fun contains(self: &TagRegistry, tag: &String): bool {
    iota::vec_map::contains(&self.tag_map, tag)
}

/// Returns the combined usage count (sum of role and record usages) for `tag`.
///
/// Returns `option::none()` when `tag` is not in the registry.
public(package) fun usage_count(self: &TagRegistry, tag: &String): Option<u64> {
    if (self.tag_map.contains(tag)) {
        option::some(*self.tag_map.get(tag))
    } else {
        option::none()
    }
}

/// Increments the combined usage count for `tag` by one.
///
/// Has no effect when `tag` is not in the registry.
public(package) fun increment_usage_count(self: &mut TagRegistry, tag: &String) {
    if (self.tag_map.contains(tag)) {
        let counters = vec_map::get_mut(&mut self.tag_map, tag);
        *counters = *counters + 1;
    };
}

/// Decrements the combined usage count for `tag` by one.
///
/// Has no effect when `tag` is not in the registry.
public(package) fun decrement_usage_count(self: &mut TagRegistry, tag: &String) {
    if (self.tag_map.contains(tag)) {
        let counters = vec_map::get_mut(&mut self.tag_map, tag);
        *counters = *counters - 1;
    };
}

/// Checks whether `tag` is currently referenced by any record or role.
///
/// Returns `false` when `tag` is not in the registry or when its combined usage
/// count is zero.
public(package) fun is_in_use(self: &TagRegistry, tag: &String): bool {
    (*self.usage_count(tag).borrow_with_default(&0)) > 0
}

// ----------- RoleMap related -------

/// Checks whether the role associated with `cap` allows the given record `tag`.
///
/// Looks up the `RoleTags` stored as role-data for `cap`'s role and tests whether
/// `tag` is part of that role's allowlist.
///
/// Returns `true` when the role has `RoleTags` whose set contains `tag`, otherwise
/// `false` (including when the role has no `RoleTags`).
public(package) fun role_allows(
    roles: &RoleMap<Permission, RoleTags>,
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
