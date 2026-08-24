# IOTA Proof of Inclusion WASM Examples

These examples construct and verify portable IOTA Proof of Inclusion proofs
with the `@iota/poi-wasm` Package. Each example creates fresh evidence through
Single Notarization instead of depending on fixed historical transactions.

## What Each Example Does

Every example has two distinct parts:

1. **Evidence setup:** The shared utility uses the active IOTA CLI environment
   and wallet. It initializes a Single Notarization Move Package, creates a
   `Notarization` object using Locked Notarization, and returns the related
   transaction, object, and event targets.
2. **Proof workflow:** The example selects one or more of those targets,
   constructs a proof from the configured gRPC source, and verifies the proof
   with the configured committee trust model.

On non-mainnet networks, the utility publishes the Move Package when it cannot
find an environment override or a cached deployment for the active chain. It
reuses that deployment on later runs. Mainnet always requires an explicit
Package ID.

## Prerequisites

Install the following tools before running an example:

- Node.js 20 or later;
- the IOTA CLI configured for the target network;
- `jq`, which the Package publication script uses;
- JSON-RPC and gRPC access to the same IOTA network; and
- a funded active CLI wallet, or faucet access on a non-mainnet network.

The utility signs with the active CLI wallet through `KeytoolSigner`. It does
not read a private key from an environment variable. On non-mainnet networks,
it requests faucet funds when the active wallet lacks enough gas. On mainnet,
you must fund the active wallet before running an example.

## Step 1: Select the Network and Wallet

Select an IOTA CLI environment and wallet address that belong to the network
where you want to run the example:

```bash
iota client switch --env <environment-alias>
iota client switch --address <wallet-address>
```

For localnet, launch the installed network with JSON-RPC, faucet, and gRPC
services before running an example:

```bash
iota-localnet start --with-faucet --with-grpc
```

The default localnet endpoints are `http://127.0.0.1:9000` for JSON-RPC and
`http://127.0.0.1:50051` for gRPC.

## Step 2: Configure the Example

The example utility reads the following environment variables:

| Name                       | Required                                                  | Description                                                                                                                                          |
| :------------------------- | :-------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------- |
| `IOTA_NOTARIZATION_PKG_ID` | mainnet only                                              | Existing Single Notarization Move Package on the active network. On other networks, this overrides automatic publication and the chain-scoped cache. |
| `NETWORK_GRPC_URL`         | custom networks or ports                                  | gRPC/ConnectRPC endpoint. Known public networks and standard localnet use their default endpoints.                                                   |
| `IOTA_GENESIS_PATH`        | genesis-anchored examples on localnet and custom networks | Independently trusted genesis blob. Known public networks download and cache their official genesis blobs automatically.                             |

The active CLI environment supplies the JSON-RPC endpoint. `NETWORK_GRPC_URL`
supplies proof evidence when the utility cannot infer the endpoint. Every
endpoint, Package ID, and genesis blob must belong to the same network.

Package publication and example transactions incur real gas costs. The utility
never requests faucet funds or publishes a Package automatically on mainnet.

## Step 3: Build and Run

Install dependencies and build the Proof of Inclusion Package:

```bash
npm install
npm run build:nodejs
```

Run a localnet example with the genesis blob created by `iota-localnet`:

```bash
IOTA_GENESIS_PATH="$HOME/.iota/iota_config/genesis.blob" \
npm run example:node -- 02_create_and_verify_multi_target_proof
```

Run against the active public faucet network. The utility funds the active CLI
wallet and publishes Single Notarization when necessary:

```bash
npm run example:node -- 04_create_and_verify_object_proof
```

On mainnet, select a funded CLI wallet and provide an existing Package ID:

```bash
IOTA_NOTARIZATION_PKG_ID=<mainnet-package-id> \
npm run example:node -- 05_create_and_verify_event_proof
```

## Examples

| Name                                                                                        | What the example proves                                                                    |
| :------------------------------------------------------------------------------------------ | :----------------------------------------------------------------------------------------- |
| [01_create_and_verify_transaction_proof](./src/01_create_and_verify_transaction_proof.ts)   | The transaction that created a `Notarization` object using Locked Notarization.            |
| [02_create_and_verify_multi_target_proof](./src/02_create_and_verify_multi_target_proof.ts) | The creation transaction, resulting `Notarization` object, and emitted event in one proof. |
| [03_reuse_verifier_for_multiple_proofs](./src/03_reuse_verifier_for_multiple_proofs.ts)     | Two creation transactions while reusing one verifier and its committee cache.              |
| [04_create_and_verify_object_proof](./src/04_create_and_verify_object_proof.ts)             | A freshly created `Notarization` object, starting from only its object ID.                 |
| [05_create_and_verify_event_proof](./src/05_create_and_verify_event_proof.ts)               | A fresh `LockedNotarizationCreated` event, starting from only its event ID.                |

## Committee Trust

The gRPC endpoint supplies transaction, checkpoint, and committee-transition
evidence. That evidence remains untrusted until verification succeeds.

Examples 01, 02, and 03 use genesis-anchored committee resolution. The utility
downloads and caches the official genesis blob for mainnet, testnet, and
devnet. Set `IOTA_GENESIS_PATH` for localnet and custom networks.

Examples 04 and 05 use trusted-node resolution so they can focus on
target-driven object and event discovery. In this mode, the selected gRPC node
is inside the verifier's trust boundary.

Obtain custom genesis blobs independently, verify that each blob belongs to the
selected network, and do not accept a trust anchor from the same untrusted party
that supplies the proof. Reuse a verifier when checking multiple proofs so it
can retain resolved committees.
