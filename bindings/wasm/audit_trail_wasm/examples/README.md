# IOTA Audit Trail WASM Examples

The examples in this folder demonstrate how to use the `@iota/audit-trail` package.

## Environment

Set the following environment variables before running the node examples:

| Name                        | Required            | Description                                           |
| --------------------------- | ------------------- | ----------------------------------------------------- |
| `IOTA_AUDIT_TRAIL_PKG_ID`   | yes                 | Package ID of the deployed `audit_trail` Move package |
| `IOTA_TF_COMPONENTS_PKG_ID` | local/custom setups | Package ID of the deployed `TfComponents` package     |
| `NETWORK_URL`               | yes                 | RPC URL of the IOTA node                              |
| `NETWORK_NAME_FAUCET`       | local/test networks | Faucet alias used by `@iota/iota-sdk`                 |

## Run

Install dependencies and build the package:

```bash
npm install
npm run build
```

Run an example:

```bash
IOTA_AUDIT_TRAIL_PKG_ID=<audit-trail-pkg-id> \
IOTA_TF_COMPONENTS_PKG_ID=<tf-components-pkg-id> \
NETWORK_URL=http://127.0.0.1:9000 \
npm run example:node -- 01_create_audit_trail
```

Available examples:

### Core

| Name                          | Description                                                                                                                                       |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `01_create_audit_trail`       | Creates an audit trail, defines a RecordAdmin role, and issues a capability for it                                                                |
| `02_add_and_read_records`     | Adds follow-up records, reads them individually and through paginated reads                                                                       |
| `03_update_metadata`          | Updates and clears mutable metadata while preserving immutable metadata via a MetadataAdmin role                                                  |
| `04_configure_locking`        | Configures write and delete locks, demonstrates that locks block record creation                                                                  |
| `05_manage_access`            | Creates and updates a role, then demonstrates constrained capability issuance, revoke and destroy flows, denylist cleanup, and final role removal |
| `06_delete_records`           | Deletes individual records and batch-deletes remaining records                                                                                    |
| `07_access_read_only_methods` | Reads trail metadata, record counts, pagination, and lock status                                                                                  |
| `08_delete_audit_trail`       | Shows that non-empty trails cannot be deleted, batch-deletes records, then deletes the trail                                                      |

### Advanced

| Name                        | Description                                                                            |
| --------------------------- | -------------------------------------------------------------------------------------- |
| `09_tagged_records`         | Uses role tags and address-bound capabilities to restrict who may add tagged records   |
| `10_capability_constraints` | Shows address-bound capability use and how revocation immediately blocks future writes |
| `11_manage_record_tags`     | Delegates tag management, adds/removes tags, shows that in-use tags cannot be removed  |

### Real-World

| Name                   | Description                                                                                                                                  |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `01_customs_clearance` | Models customs clearance with role-tag restrictions, delegated capabilities, denied inspection writes, and a final write lock                |
| `02_clinical_trial`    | Models a clinical trial with time-constrained capabilities, mid-study tag addition, deletion windows, time-locks, and regulator verification |
