# IOTA Audit Trail Examples

The following code examples demonstrate how to use IOTA Audit Trails for creating structured, role-based audit logs on the IOTA network.

## Prerequisites

Examples can be run against:

- A local IOTA node
- An existing network, e.g., the IOTA testnet

When setting up a local node, you'll need to publish an audit trail package as described in the IOTA documentation. You'll also need to provide environment variables for your locally deployed audit trail package to run the examples against the local node.

If running the examples on `testnet`, use the appropriate package IDs for the testnet deployment.

In case of running the examples against an existing network, this network needs to have a faucet to fund your accounts (the IOTA testnet (`https://api.testnet.iota.cafe`) supports this), and you need to specify this via `API_ENDPOINT`.

## Environment Variables

You'll need one or more of the following environment variables depending on your setup:

| Name                      | Required for local node | Required for testnet | Required for other node |
| ------------------------- | :---------------------: | :------------------: | :---------------------: |
| IOTA_AUDIT_TRAIL_PKG_ID   |            x            |          x           |            x            |
| IOTA_TF_COMPONENTS_PKG_ID |            x            |                      |                         |
| API_ENDPOINT              |                         |          x           |            x            |

> **Note:** On localnet both `IOTA_AUDIT_TRAIL_PKG_ID` and `IOTA_TF_COMPONENTS_PKG_ID` resolve to the same package ID because the TfComponents dependency is published together with the audit trail package.

## Running Examples

The publish script prints the required `export` statements, so use `eval` to set the variables in one step:

```bash
eval $(./audit-trail-move/scripts/publish_package.sh)
```

Then run a specific example:

```bash
cargo run --release --example <example-name>
```

For instance, to run the `01_create_audit_trail` example:

```bash
eval $(./audit-trail-move/scripts/publish_package.sh)
cargo run --release --example 01_create_audit_trail
```

To pass the variables inline instead:

```bash
IOTA_AUDIT_TRAIL_PKG_ID=0x... IOTA_TF_COMPONENTS_PKG_ID=0x... cargo run --release --example 01_create_audit_trail
```

## Examples

| Name                                                                                                                                    | Information                                                                                                                                        |
| :-------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------- |
| [01_create_audit_trail](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/01_create_audit_trail.rs)             | Creates an audit trail, defines a `RecordAdmin` role using the Admin capability, and issues a capability for it.                                   |
| [02_add_and_read_records](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/02_add_and_read_records.rs)         | Adds follow-up records to a trail, then loads them back individually and through paginated reads.                                                  |
| [03_update_metadata](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/03_update_metadata.rs)                   | Updates and clears the trail's mutable metadata while preserving immutable metadata.                                                               |
| [04_configure_locking](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/04_configure_locking.rs)               | Configures write and delete locks, then shows how those rules affect record creation.                                                              |
| [05_manage_access](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/05_manage_access.rs)                       | Creates and updates a role, then demonstrates constrained capability issuance, revoke and destroy flows, denylist cleanup, and final role removal. |
| [06_delete_records](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/06_delete_records.rs)                     | Deletes an individual record and then removes the remaining records in a batch.                                                                    |
| [07_access_read_only_methods](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/07_access_read_only_methods.rs) | Reads back trail metadata, locking state, record counts, and paginated record data.                                                                |
| [08_delete_audit_trail](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/08_delete_audit_trail.rs)             | Empties a trail and then deletes it, showing that non-empty trails cannot be removed.                                                              |

## Advanced Examples

| Name                                                                                                                                         | Information                                                                             |
| :------------------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------- |
| [09_tagged_records](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/advanced/09_tagged_records.rs)                 | Uses role tags and address-bound capabilities to restrict who may add tagged records.   |
| [10_capability_constraints](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/advanced/10_capability_constraints.rs) | Shows address-bound capability use and how revocation immediately blocks future writes. |
| [11_manage_record_tags](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/advanced/11_manage_record_tags.rs)         | Delegates record-tag administration and shows that in-use tags cannot be removed.       |

## Real-World Examples

| Name                                                                                                                                               | Information                                                                                                                                                                                     |
| :------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [01_customs_clearance](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/real-world/01_customs_clearance.rs)               | Models customs clearance with role-tag restrictions, delegated capabilities, denied inspection writes, and a final write lock.                                                                  |
| [02_clinical_trial](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/real-world/02_clinical_trial.rs)                     | Models a Phase III clinical trial with time-constrained capabilities, mid-study tag additions, deletion-window enforcement, time-locked datasets, and read-only regulator verification.         |
| [03_digital_product_passport](https://github.com/iotaledger/notarization/tree/main/examples/audit-trail/real-world/03_digital_product_passport.rs) | Models a Digital Product Passport for an e-bike battery with lifecycle-scoped actors, technician access approval, an annual maintenance event, and documented Lifecycle Credit reward evidence. |

## Key Concepts

### Audit Trail

An audit trail is an on-chain object that stores an ordered sequence of records. Each trail has:

- **Immutable metadata**: Name and description set at creation, never changes
- **Updatable metadata**: A mutable string for operational status or notes
- **Record log**: An append-only sequence of records (text or binary data)
- **Role map**: Named roles with permission sets that control who can do what
- **Locking config**: Optional write, delete-record, and delete-trail locks

### Role-Based Access Control

Access to trail operations is controlled via roles and capabilities:

- **Roles** define a named set of permissions (e.g., `RecordAdmin` with `AddRecord`, `DeleteRecord`, `CorrectRecord`)
- **Capabilities** are on-chain objects issued for a role and held in a wallet — possession of a capability grants the associated permissions on a specific trail
- The trail creator automatically receives an **Admin** capability granting full administrative control (role management, capability issuance, tag management, etc.)

### Permission Sets

`PermissionSet` convenience constructors cover common role configurations:

| Constructor                    | Permissions granted                                                                                      |
| :----------------------------- | :------------------------------------------------------------------------------------------------------- |
| `admin_permissions()`          | AddRoles, UpdateRoles, DeleteRoles, AddCapabilities, RevokeCapabilities, AddRecordTags, DeleteRecordTags |
| `record_admin_permissions()`   | AddRecord, DeleteRecord, CorrectRecord                                                                   |
| `locking_admin_permissions()`  | UpdateLockingConfig (and all sub-variants)                                                               |
| `cap_admin_permissions()`      | AddCapabilities, RevokeCapabilities                                                                      |
| `tag_admin_permissions()`      | AddRecordTags, DeleteRecordTags                                                                          |
| `metadata_admin_permissions()` | UpdateMetadata, DeleteMetadata                                                                           |

### Capability Constraints

When issuing a capability, `CapabilityIssueOptions` allows restricting its use:

- **`issued_to`**: Bind the capability to a specific wallet address
- **`valid_from_ms`**: The capability is not valid before this Unix timestamp (ms)
- **`valid_until_ms`**: The capability expires after this Unix timestamp (ms)

### Locking

Trails support three independent lock dimensions:

- **Write lock** (`TimeLock`): Prevents new records from being added
- **Delete-record window** (`LockingWindow`): Time-based or count-based window during which a record can be deleted after creation
- **Delete-trail lock** (`TimeLock`): Prevents the trail itself from being destroyed

`TimeLock` variants: `None`, `UnlockAt(u32)`, `UnlockAtMs(u64)`, `UntilDestroyed`, `Infinite`.

## Example Scenarios

### Audit Log Workflow

1. **Create** a trail with immutable metadata and an initial record
2. **Define roles** (e.g., `RecordAdmin`, `Auditor`) using the Admin capability
3. **Issue capabilities** to operators or auditors
4. **Add records** using a RecordAdmin capability
5. **Query** records and trail state at any time

### Compliance Use Cases

- **Locked write windows** to prevent retroactive record insertion
- **Delete-record windows** to allow corrections within a time limit, then freeze
- **Role separation** to enforce least-privilege access (auditors can read, operators can write)
- **Bound capabilities** to tie a capability to a specific operator address

## Best Practices

1. **Separate roles by responsibility**: Use distinct roles for writing records, managing locking, and administering capabilities
2. **Bind capabilities to addresses**: Use `issued_to` to prevent capability sharing
3. **Set validity windows**: Use `valid_from_ms` / `valid_until_ms` to limit capability lifetime
4. **Use record tags**: Define a tag registry on the trail and restrict roles to specific tags for finer-grained access control
5. **Plan locking upfront**: Locking configuration is easier to set at creation than to change later

## Security Considerations

- Audit trails and their records are publicly readable on the blockchain
- Private keys control which capabilities a wallet holds
- Bound capabilities (`issued_to`) prevent transfer and unauthorized use
- Delete-trail locks ensure data retention requirements are met
- Revoking a capability adds it to the trail's revoked-capability registry, blocking future use

For more detailed information about IOTA Audit Trail concepts and advanced usage, refer to the official IOTA documentation.
