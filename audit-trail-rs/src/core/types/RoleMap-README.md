# Role-Based Access Control for Audit Trails

Audit trails provide an access control registry (a.k.a. `RoleMap`), defining who may perform which
operations by combining two primitives:

- **Roles** — named permission sets stored on the trail.
- **Capabilities** — on-chain objects held by users, each linked to one role.

Every operation on a trail (adding a record, deleting a role, revoking a
capability, …) requires the caller to present a `Capability`. The audit trail
validates the capability before allowing the operation.

---

## Concepts

### Roles

A role is a named and configurable set of `Permission` values, for example:

| Role name      | Permissions                                                                                              |
| :------------- | :------------------------------------------------------------------------------------------------------- |
| `Admin`        | AddRoles, UpdateRoles, DeleteRoles, AddCapabilities, RevokeCapabilities, AddRecordTags, DeleteRecordTags |
| `RecordAdmin`  | AddRecord, DeleteRecord, CorrectRecord                                                                   |
| `LockingAdmin` | UpdateLockingConfig (and sub-variants)                                                                   |
| `Auditor`      | _(read-only — no write permissions needed)_                                                              |

Roles are identified by a unique string name within the trail. Multiple
capabilities can be issued for the same role, to allow users or services to share
that access level.

Roles may optionally carry a `RoleTags` allowlist (see [Record Tags](#record-tags-and-roletags)).

### Capabilities

A `Capability` is an on-chain object owned by a wallet address. It records:

| Field         | Meaning                                                            |
| :------------ | :----------------------------------------------------------------- |
| `target_key`  | The `ObjectID` of the trail this capability is valid for.          |
| `role`        | The role name — determines which permissions the holder has.       |
| `issued_to`   | Optional address binding; only that address may present the cap.   |
| `valid_from`  | Optional Unix-ms timestamp before which the cap is not yet active. |
| `valid_until` | Optional Unix-ms timestamp after which the cap expires.            |

Possessing a capability does **not** automatically grant access. The audit trail
validates all fields above on every call before the operation is executed.

### The Admin Role

When a trail is created, the access control registry is initialized with exactly one role —
the **initial admin role** (named `"Admin"`). A corresponding capability
object is minted and transferred to the trail creator (or a custom address
supplied via `with_admin`).

The Admin role is protected by two invariants:

1. It can **never be deleted**.
2. Although its permission set can be updated, it needs to include a minimum set of
   permissions to manage the trail's access control (AddRoles, UpdateRoles, DeleteRoles,
   AddCapabilities, RevokeCapabilities). Removing any of these permissions from the Admin
   role will fail.

Initial admin capabilities are tracked in `initial_admin_cap_ids` and must be
managed through dedicated entry-points (`revoke_initial_admin_capability`,
`destroy_initial_admin_capability`).

---

## Lifecycle Example

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

Please note: Revoked capability objects still exist on-chain but will be rejected by
`assert_capability_valid`. The holder can no longer use it.

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

Tags are string labels that can be attached to individual records. They are
managed through a **tag registry** on the trail: a tag must be registered
before it can be used on a record or referenced by a role.

### Why use tags?

Tags enable fine-grained access control beyond simple permission checks. For
example, a legal department may only be allowed to access records tagged
`"legal"`, while the finance team works with records tagged `"finance"`.

### How tags interact with roles

A role may carry an optional `RoleTags` allowlist. When a capability holder
adds a record with a tag, the audit trail checks that:

1. The tag is registered in the trail's tag registry.
2. The role associated with the capability includes the requested tag in its
   `RoleTags` allowlist.

If either check fails the transaction is rejected.

The same checks apply when a record having a tag is updated or deleted.

Please note:

- Tags only restrict the use of tagged records to roles that explicitly
  grant access to those tags in the associated `RoleTags` allowlist.
- Tags do not grant access permission themselves. A role still needs the relevant
  permissions (e.g. `AddRecord`) to perform operations on tagged records.
- A role without any `RoleTags` can operate on any record not having tags, as long
  as it has the necessary permissions.

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

Every operation on a trail calls `assert_capability_valid` before executing.
The checks run in the order listed below; the transaction aborts on the
**first** failing check.

### 1 — `ECapabilityTargetKeyMismatch`

The capability's `target_key` must match the `target_key` of the RoleMap
(which is typically the `ObjectID` of the audit trail). This prevents a
capability issued for one trail from being used on a different trail.

### 2 — `ERoleDoesNotExist`

The role name stored in the capability must still exist in the RoleMap. If
an admin deleted the role after the capability was issued, the capability
becomes unusable — even though it was never explicitly revoked.

### 3 — `ECapabilityPermissionDenied`

The role's current permission set must contain the permission required by the
operation being performed. For example, calling `add_record` requires the
`AddRecord` permission. If the role was updated after the capability was
issued and the required permission was removed, existing capabilities for
that role will start failing this check.

### 4 — `ECapabilityHasBeenRevoked`

The capability's ID must **not** appear in the `revoked_capabilities`
denylist. A capability that has been revoked via `revoke_capability` (or
`revoke_initial_admin_capability`) is permanently rejected, even if it is
still within its validity window. See
[Managing Revoked Capabilities](#managing-revoked-capabilities) for details.

### 5 — `ECapabilityTimeConstraintsNotMet`

This check only runs when the capability has a `valid_from` and/or
`valid_until` field set. The current on-chain clock time must satisfy:

- `valid_from`: current time **>=** `valid_from` (the capability is not yet
  active before this timestamp).
- `valid_until`: current time **<=** `valid_until` (the capability has
  expired after this timestamp).

If neither field is set, this check is skipped entirely and the capability is
considered valid at any point in time.

### 6 — `ECapabilityIssuedToMismatch`

This check only runs when the capability has a non-empty `issued_to` field.
The address of the transaction sender must match the `issued_to` address
stored in the capability. This binds the capability to a specific wallet,
preventing it from being used by anyone else even if the on-chain object is
transferred.

If `issued_to` is not set, any holder of the capability object may use it.

### 7 — `ERecordTagNotDefined` / `ERecordTagNotAllowed`

This check is performed by the audit trail **after** all `RoleMap` checks
(1–6) have passed. It only applies to record operations (add, correct,
delete) that involve a tagged record.

When a record carries a tag, two additional conditions must hold:

1. The tag must be registered in the trail's **tag registry**
   (`ERecordTagNotDefined`).
2. The role associated with the capability must include the tag in its
   `RoleTags` allowlist (`ERecordTagNotAllowed`). A role without any
   `RoleTags` is **not** permitted to operate on tagged records.

If the record has no tag, this check is skipped. See
[Record Tags and RoleTags](#record-tags-and-roletags) for a full explanation
and examples.

### Summary

| #  | Check                   | Error                                           | Skippable                                          |
| :- | :---------------------- | :---------------------------------------------- | :------------------------------------------------- |
| 1  | `target_key` mismatch   | `ECapabilityTargetKeyMismatch`                  | No                                                 |
| 2  | Role does not exist     | `ERoleDoesNotExist`                             | No                                                 |
| 3  | Permission not in role  | `ECapabilityPermissionDenied`                   | No                                                 |
| 4  | ID in revoked denylist  | `ECapabilityHasBeenRevoked`                     | No                                                 |
| 5  | Outside validity window | `ECapabilityTimeConstraintsNotMet`              | Yes — only if `valid_from` or `valid_until` is set |
| 6  | `issued_to` mismatch    | `ECapabilityIssuedToMismatch`                   | Yes — only if `issued_to` is set                   |
| 7  | Record tag not allowed  | `ERecordTagNotDefined` / `ERecordTagNotAllowed` | Yes — only for record operations on tagged records |

---

## Managing Revoked Capabilities

### The `revoked_capabilities` Denylist

When a capability is revoked it is **not deleted from the chain** — the
on-chain `Capability` object still exists in the holder's wallet. Instead,
the capability's ID is added to a **denylist** stored inside the audit trail.
During every call to an access restricted audit trail function, the internally
called `assert_capability_valid` function checks the denylist and rejects any capability whose
ID appears in it (error `ECapabilityHasBeenRevoked`).

The denylist approach (as opposed to an allowlist of all issued capabilities)
was chosen deliberately: it keeps on-chain storage proportional to the number
of _currently revoked_ capabilities rather than the total number ever issued.
This is important for deployments that issue large numbers of capabilities over
time.

Each denylist entry maps a revoked capability ID to a `valid_until` timestamp
(Unix milliseconds). If the revoked capability had no `valid_until` field, the
stored value is `0`, which signals "no expiry — keep in the denylist
indefinitely".

### How Time-Restricted Capabilities Affect Management

Capabilities can carry optional `valid_from` and `valid_until` timestamps.
These fields are enforced by the internally used `assert_capability_valid`:
a capability whose
time window has not yet started or has already passed is rejected with
`ECapabilityTimeConstraintsNotMet`, regardless of whether it appears in the
denylist.

This has an important consequence for revocation: **once a capability's
`valid_until` timestamp has passed, the capability is naturally expired and
can no longer be used — even if it was never explicitly revoked.** Its
denylist entry therefore becomes redundant and can be safely removed.

The `cleanup_revoked_capabilities` function exploits this property. It
iterates through the denylist and removes every entry whose stored
`valid_until` value is **non-zero** and **less than** the current clock time.
Entries with `valid_until == 0` (capabilities that were issued without an
expiry or where the revoker did not supply the `valid_until` value during the
`revoke_capability` call) are kept because the corresponding capabilities never
expire on their own.

**Best practice:** always set a `valid_until` when issuing capabilities.
Even a generous validity window (e.g. one year) ensures that the
corresponding denylist entry can be automatically cleaned up after the
capability expires, rather than occupying storage indefinitely.

### Off-Chain Tracking Requirements

Because the audit trail uses a denylist and not an allowlist, it does **not**
maintain an on-chain registry of all issued capabilities. Tracking every
issued capability on-chain would increase storage costs and slow down
validity checks.

This design shifts the bookkeeping responsibility to the user:

1. **Maintain an off-chain registry of every issued capability**, storing at
   least the capability `ID`, the `role` it was issued for, the `issued_to`
   address (if any), and the `valid_from` / `valid_until` timestamps.
2. **When revoking**, supply the correct capability ID and its `valid_until`
   value (via the `cap_to_revoke_valid_until` parameter). The
   `revoke_capability` function does **not** verify that the supplied ID
   actually refers to a real, previously-issued capability — if you pass a
   random ID, it will be silently added to the denylist without error.
   Accurate off-chain records are therefore essential.
3. **Track which capabilities have been revoked or destroyed** so you do not
   attempt to revoke the same capability twice (which would abort with
   `ECapabilityToRevokeHasAlreadyBeenRevoked`).

The off-chain capability registry can also be used to manage capability renewal:
when a capability is about to expire, a new capability is automatically issued for the
holder with an updated validity window. The old capability can be revoked or destroyed
at the same time. This process can be fully automated by a background service that
monitors capability expirations and performs renewals as needed.

For deployments that only issue a small number of capabilities, a simplified
approach is acceptable: track only the issued capability IDs and pass
`None` for `cap_to_revoke_valid_until` when revoking capabilities using the
`revoke_capability` function. The trade-off is that
those denylist entries will never be automatically cleaned up — they persist
until the capability object is explicitly destroyed.

### Cleaning Up the Denylist

Over time the denylist can accumulate entries for capabilities that have
already naturally expired. The `cleanup_revoked_capabilities` function
removes these stale entries:

1. It walks through every entry in the `revoked_capabilities` linked table.
2. For each entry with a **non-zero** `valid_until` value that is **less than**
   the current on-chain clock time, the entry is removed.
3. Entries with `valid_until == 0` are skipped — they represent capabilities
   that have no natural expiry and must remain on the denylist until the
   capability object itself is destroyed (via `destroy_capability`).

The cleanup operation requires a capability with the `RevokeCapabilities`
permission.

**Recommendations for keeping the denylist short:**

- Always provide the `cap_to_revoke_valid_until` value that matches the `valid_until` of the
  revoked capability when revoking a capability so that
  the entry becomes eligible for automatic cleanup.
- Call `cleanup_revoked_capabilities` periodically (e.g. as a maintenance
  transaction) to reclaim storage.
- When a revoked capability is no longer needed at all, have the holder call
  `destroy_capability` to delete the on-chain object. Destroying a
  capability also removes it from the denylist if it was listed there.

---

## Permission Sets

`PermissionSet` provides convenience constructors for common role profiles:

| Constructor                    | Permissions                                                                                              |
| :----------------------------- | :------------------------------------------------------------------------------------------------------- |
| `admin_permissions()`          | AddRoles, UpdateRoles, DeleteRoles, AddCapabilities, RevokeCapabilities, AddRecordTags, DeleteRecordTags |
| `record_admin_permissions()`   | AddRecord, DeleteRecord, CorrectRecord                                                                   |
| `locking_admin_permissions()`  | UpdateLockingConfig (and all sub-variants)                                                               |
| `cap_admin_permissions()`      | AddCapabilities, RevokeCapabilities                                                                      |
| `tag_admin_permissions()`      | AddRecordTags, DeleteRecordTags                                                                          |
| `metadata_admin_permissions()` | UpdateMetadata, DeleteMetadata                                                                           |

Please note:

- These constructors are just for convenience and do not enforce any invariants.
  For example, you could (not recommended) create a role named `NormalUser` with
  `PermissionSet::admin_permissions()`.
- You can create custom permission sets by constructing a `PermissionSet` with
  an arbitrary combination of permissions.
