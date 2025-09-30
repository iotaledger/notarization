![banner](https://github.com/iotaledger/notarization/raw/HEAD/.github/banner_notarization.png)

## IOTA Notarization Examples

The following code examples demonstrate how to use the IOTA Notarization Wasm bindings in JavaScript/TypeScript.

The examples are written in TypeScript and can be run with Node.js.

### Prerequisites

Examples can be run against

- a local IOTA node
- or an existing network, e.g. the IOTA testnet

When setting up the local node, you'll also need to publish a notarization package as described in
[Local Network Setup](https://docs.iota.org/developer/iota-notarization/getting-started/local-network-setup) in the documentation portal.
You'll also need to provide an environment variable `IOTA_NOTARIZATION_PKG_ID` set to the package-id of your locally deployed
notarization package, to be able to run the examples against the local node.

In case of running the examples against an existing network, this network needs to have a faucet to fund your accounts (the IOTA testnet (`https://api.testnet.iota.cafe`) supports this), and you need to specify this via `NETWORK_URL`.

The examples require you to have the node you want to use in the iota clients "envs" (`iota client env`) configuration. If this node is configured as `localnet`, you don't have to provide it when running the examples, if not, provide its name as `NETWORK_NAME_FAUCET`. The table below assumes - in case you're running a local node - you have it configured as `localnet` in your IOTA clients "env" setting.

### Environment variables

Summarizing the last point, you'll need one or more of the following environment variables:

| Name                     | Required for local node | Required for testnet | Required for other node |       Comment        |
| ------------------------ | :---------------------: | :------------------: | :---------------------: | :------------------: |
| IOTA_NOTARIZATION_PKG_ID |            x            |                      |            x            |                      |
| NETWORK_URL              |                         |          x           |            x            |                      |
| NETWORK_NAME_FAUCET      |                         |          x           |            x            | see assumption above |

### Node.js

Install the dependencies:

```bash
npm install
```

Build the bindings:

```bash
npm run build
```

Then, run an example using the following command, environment variables depend on your setup, see [Environment variables](#environment-variables).

```bash
IOTA_NOTARIZATION_PKG_ID=0x222741bbdff74b42df48a7b4733185e9b24becb8ccfbafe8eac864ab4e4cc555 npm run example:node -- <example-name>
```

For instance, to run the `0_create_did` example with the following (environment variables depend on you setup, see [Environment variables](#environment-variables)):

```bash
IOTA_NOTARIZATION_PKG_ID=0x222741bbdff74b42df48a7b4733185e9b24becb8ccfbafe8eac864ab4e4cc555 npm run example:node -- 0_create_did
```

## Basic Examples

The following examples are available:

| Name                                                                                                                                                            | Information                                                                                           |
| :-------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------------------------------------------------- |
| [01_create_locked](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/01_create_locked.ts)                       | Demonstrates how to create a a new locked notarization.                                               |
| [02_create_dynamic](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/02_create_dynamic.ts)                     | Demonstrates how to create a a new dynamic notarization.                                              |
| [03_update_dynamic](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/03_update_dynamic.ts)                     | Demonstrates how to update a dynamic notarization.                                                    |
| [04_destroy_notarization](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/04_destroy_notarization.ts)         | Demonstrates how to destroy a Notarization.                                                           |
| [05_update_state](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/05_update_state.ts)                         | Demonstrates how to update the state of a Notarization.                                               |
| [06_update_metadata](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/06_update_metadata.ts)                   | Demonstrates how to update the metadata of a Notarization.                                            |
| [07_transfer_notarization](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/07_transfer_notarization.ts)       | Demonstrates how to transfer a dynamic Notarization and transferring a locked Notarization will fail. |
| [08_access_read_only_methods](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/08_access_read_only_methods.ts) | Demonstrates read-only methods for notarization inspection.                                           |

## Real-World Examples

The following examples demonstrate practical use cases with proper field usage:

| Name                                                                                                                                                                        | Information                                                                        |
| :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------- |
| [01_real_world_iot_weather_station](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/real-world/01_iot_weather_station.ts) | IoT weather station using dynamic notarization for continuous sensor data updates. |
| [02_real_world_legal_contract](https://github.com/iotaledger/notarization/tree/main/bindings/wasm/notarization_wasm/examples/src/real-world/02_legal_contract.ts)           | Legal contract using locked notarization for immutable document hash attestation.  |

<!--

## Browser

While the examples should work in a browser environment, we do not provide browser examples yet.

-->
