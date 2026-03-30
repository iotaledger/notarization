# IOTA Audit Trails WASM Examples

The examples in this folder demonstrate the Core MVP flow of the `@iota/audit-trails` package:

- create a trail
- fetch a trail
- add and page records
- delete records in batch

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
npm run example:node -- 01_create_trail
```

Available examples:

- `01_create_trail`
- `02_fetch_trail`
- `03_add_and_list_records`
- `04_delete_records_batch`
