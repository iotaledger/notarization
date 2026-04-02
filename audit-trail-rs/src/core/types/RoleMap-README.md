# RoleMap — Role-Based Access Control for Audit Trails

A `RoleMap` is the access control registry embedded in every audit trail.
It defines who may perform which operations by combining two primitives:

- **Roles** — named permission sets stored on the trail.
- **Capabilities** — on-chain objects held by users, each linked to one role.

Every operation on a trail (adding a record, deleting a role, revoking a
capability, …) requires the caller to present a `Capability`.  The `RoleMap`
validates the capability before allowing the operation.

---

## Concepts

### Roles

A role is a named set of `Permission` values, for example:

| Role name       | Permissions                                 |
|:----------------|:--------------------------------------------|
| `Admin`         | AddRoles, UpdateRoles, DeleteRoles, AddCapabilities, RevokeCapabilities, AddRecordTags, DeleteRecordTags |
| `RecordAdmin`   | AddRecord, DeleteRecord, CorrectRecord       |
| `LockingAdmin`  | UpdateLockingConfig (and sub-variants)       |
| `Auditor`       | *(read-only — no write permissions needed)*  |

Roles are identified by a unique string name within the trail.  Multiple
capabilities can be issued for the same role, one per user or service that
should share that access level.

Roles may optionally carry a `RoleTags` allowlist (see [Record Tags](#record-tags-and-roletags)).

### Capabilities

A `Capability` is an on-chain object owned by a wallet address.  It records:

| Field         | Meaning                                                               |
|:--------------|:----------------------------------------------------------------------|
| `target_key`  | The `ObjectID` of the trail this capability is valid for.             |
| `role`        | The role name — determines which permissions the holder has.          |
| `issued_to`   | Optional address binding; only that address may present the cap.      |
| `valid_from`  | Optional Unix-ms timestamp before which the cap is not yet active.    |
| `valid_until` | Optional Unix-ms timestamp after which the cap expires.               |

Possessing a capability does **not** automatically grant access.  The `RoleMap`
validates all fields above on every call before the operation is executed.

### The Admin Role

When a trail is created, the `RoleMap` initialises with exactly one role —
the **initial admin role** (named `"Admin"`).  A corresponding capability
object is minted and transferred to the trail creator (or a custom address
supplied via `with_admin`).

The Admin role is protected by two invariants:
1. It can **never be deleted**.
2. Its permission set can only be updated to a set that still includes all
   configured role- and capability-admin permissions.

Initial admin capabilities are tracked in `initial_admin_cap_ids` and must be
managed through dedicated entry-points (`revoke_initial_admin_capability`,
`destroy_initial_admin_capability`).

---

## Lifecycle

### 1 — Trail is created

```
Trail creator  ──create_trail()──►  AuditTrail (shared object)
                                         │
                                         └── RoleMap
                                               ├── roles: { "Admin" → [AddRoles, …] }
                                               ├── initial_admin_role_name: "Admin"
                                               └── initial_admin_cap_ids: { cap_id }
                    ◄── Admin Capability (owned object, transferred to creator)
```

### 2 — Admin defines additional roles

The trail creator (Admin capability holder) defines a `RecordAdmin` role:

```
Admin Capability + create_role("RecordAdmin", [AddRecord, DeleteRecord, CorrectRecord])
    ──►  RoleMap.roles: { "Admin" → […], "RecordAdmin" → [AddRecord, DeleteRecord, CorrectRecord] }
```

### 3 — Admin issues capabilities to operators

```
Admin Capability + issue_capability("RecordAdmin", issued_to = operator_address)
    ──►  RecordAdmin Capability (owned object, transferred to operator)
```

### 4 — Operator uses their capability

```
RecordAdmin Capability + add_record(trail, data)
    ──►  RoleMap.assert_capability_valid(cap, AddRecord)   // validated
    ──►  Record appended to trail
```

### 5 — Admin revokes a capability

```
Admin Capability + revoke_capability(cap_id, valid_until)
    ──►  RoleMap.revoked_capabilities: { cap_id → valid_until_ms }
```

The capability object still exists on-chain but is rejected by
`assert_capability_valid`.  The holder can no longer use it.

---

## Rust API Quick Reference

### Creating a trail and obtaining the Admin capability

```rust
use audit_trail::core::types::{Data, InitialRecord, ImmutableMetadata};

let created = client
    .create_trail()
    .with_trail_metadata(ImmutableMetadata::new("My Trail".into(), None))
    .with_initial_record(InitialRecord::new(Data::text("first entry"), None, None))
    .finish()
    .build_and_execute(&client)
    .await?
    .output; // TrailCreated { trail_id, creator, timestamp }

// The Admin capability is now in the creator's wallet.
```

### Defining a new role

```rust
use audit_trail::core::types::PermissionSet;

client
    .trail(created.trail_id)
    .access()
    .for_role("RecordAdmin")
    .create(PermissionSet::record_admin_permissions(), None)
    .build_and_execute(&client)
    .await?;
```

### Issuing a capability

```rust
use audit_trail::core::types::CapabilityIssueOptions;

// Unrestricted — any holder may use this capability
let cap = client
    .trail(created.trail_id)
    .access()
    .for_role("RecordAdmin")
    .issue_capability(CapabilityIssueOptions::default())
    .build_and_execute(&client)
    .await?
    .output; // CapabilityIssued { capability_id, target_key, role, … }

// Address-bound and time-limited
let cap = client
    .trail(created.trail_id)
    .access()
    .for_role("RecordAdmin")
    .issue_capability(CapabilityIssueOptions {
        issued_to: Some(operator_address),
        valid_from_ms: None,
        valid_until_ms: Some(1_800_000_000_000), // expires at this Unix-ms timestamp
    })
    .build_and_execute(&client)
    .await?
    .output;
```

### Revoking a capability

```rust
client
    .trail(trail_id)
    .access()
    .revoke_capability(cap.capability_id, cap.valid_until)
    .build_and_execute(&client)
    .await?;
```

### Cleaning up the denylist

```rust
// Removes all denylist entries whose valid_until has already passed.
client
    .trail(trail_id)
    .access()
    .cleanup_revoked_capabilities()
    .build_and_execute(&client)
    .await?;
```

### Updating a role's permissions

```rust
use audit_trail::core::types::{Permission, PermissionSet};
use std::collections::HashSet;

client
    .trail(trail_id)
    .access()
    .for_role("RecordAdmin")
    .update_permissions(
        PermissionSet {
            permissions: HashSet::from([Permission::AddRecord, Permission::CorrectRecord]),
        },
        None, // no RoleTags change
    )
    .build_and_execute(&client)
    .await?;
```

### Deleting a role

```rust
client
    .trail(trail_id)
    .access()
    .for_role("RecordAdmin")
    .delete()
    .build_and_execute(&client)
    .await?;
// Note: the initial admin role ("Admin") cannot be deleted.
```

---

## Record Tags and RoleTags

Tags are string labels that can be attached to individual records.  They are
managed through a **tag registry** on the trail: a tag must be registered
before it can be used on a record or referenced by a role.

### Why use tags?

Tags enable fine-grained access control beyond simple permission checks.  For
example, a legal department may only be allowed to read records tagged
`"legal"`, while the finance team works with records tagged `"finance"`.

### How tags interact with roles

A role may carry an optional `RoleTags` allowlist.  When a capability holder
adds a record with a tag, the `RoleMap` checks that:

1. The tag is registered in the trail's tag registry.
2. The role associated with the capability includes the requested tag in its
   `RoleTags` allowlist.

If either check fails the transaction is rejected.

### Example — tagged records

```rust
// 1. Create trail with a tag registry
let created = client
    .create_trail()
    .with_record_tags(["finance", "legal"])
    .with_initial_record(InitialRecord::new(Data::text("opening entry"), None, None))
    .finish()
    .build_and_execute(&client)
    .await?
    .output;

// 2. Create a role that may only write "finance" tagged records
use audit_trail::core::types::RoleTags;

client
    .trail(created.trail_id)
    .access()
    .for_role("FinanceWriter")
    .create(
        PermissionSet { permissions: HashSet::from([Permission::AddRecord]) },
        Some(RoleTags::new(["finance"])),
    )
    .build_and_execute(&client)
    .await?;
```

A `FinanceWriter` capability holder can add records tagged `"finance"` but not
records tagged `"legal"`.

---

## Capability Validation Rules

`assert_capability_valid` rejects a capability if any of the following hold:

| Check                   | Error                               |
|:------------------------|:------------------------------------|
| `target_key` mismatch   | `ECapabilityTargetKeyMismatch`      |
| Role does not exist     | `ERoleDoesNotExist`                 |
| Permission not in role  | `ECapabilityPermissionDenied`       |
| ID in revoked denylist  | `ECapabilityHasBeenRevoked`         |
| Outside validity window | `ECapabilityTimeConstraintsNotMet`  |
| `issued_to` mismatch    | `ECapabilityIssuedToMismatch`       |

---

## Denylist Management

The `RoleMap` uses a **denylist** (not an allowlist) for revocation.  This
keeps on-chain storage proportional to the number of *currently revoked*
capabilities, not the total number ever issued.

Implications:

- **Off-chain tracking is required.** Users must maintain a record of every
  issued capability ID and its `valid_until` value so the correct ID can be
  passed to `revoke_capability`.
- **Provide `valid_until` when revoking.** The stored value lets the denylist
  entry be cleaned up automatically once it expires.
- **Call `cleanup_revoked_capabilities` periodically** to remove expired
  entries and keep storage costs low.
- Capabilities revoked without a `valid_until` stay in the denylist until
  explicitly destroyed.

---

## Permission Sets

`PermissionSet` provides convenience constructors for common role profiles:

| Constructor                   | Permissions                                                                    |
|:------------------------------|:-------------------------------------------------------------------------------|
| `admin_permissions()`         | AddRoles, UpdateRoles, DeleteRoles, AddCapabilities, RevokeCapabilities, AddRecordTags, DeleteRecordTags |
| `record_admin_permissions()`  | AddRecord, DeleteRecord, CorrectRecord                                         |
| `locking_admin_permissions()` | UpdateLockingConfig (and all sub-variants)                                     |
| `cap_admin_permissions()`     | AddCapabilities, RevokeCapabilities                                            |
| `tag_admin_permissions()`     | AddRecordTags, DeleteRecordTags                                                |
| `metadata_admin_permissions()`| UpdateMetadata, DeleteMetadata                                                 |
