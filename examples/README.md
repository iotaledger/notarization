# IOTA Notarization Examples

The following code examples demonstrate how to use IOTA Notarization for creating, managing, and interacting with notarized documents on the IOTA network.

## Prerequisites

Examples can be run against:

- A local IOTA node
- An existing network, e.g., the IOTA testnet

When setting up a local node, you'll need to publish a notarization package as described in the IOTA documentation. You'll also need to provide environment variables for your locally deployed notarization package to run the examples against the local node.

If running the examples on `testnet`, use the appropriate package IDs for the testnet deployment.

In case of running the examples against an existing network, this network needs to have a faucet to fund your accounts (the IOTA testnet (`https://api.testnet.iota.cafe`) supports this), and you need to specify this via `API_ENDPOINT`.

## Environment Variables

You'll need one or more of the following environment variables depending on your setup:

| Name                     | Required for local node | Required for testnet | Required for other node |
| ------------------------ | :---------------------: | :------------------: | :---------------------: |
| IOTA_NOTARIZATION_PKG_ID |            x            |          x           |            x            |
| API_ENDPOINT             |                         |          x           |            x            |

## Running Examples

Run an example using the following command (environment variables depend on your setup):

```bash
IOTA_NOTARIZATION_PKG_ID=0x... cargo run --example <example-name>
```

For instance, to run the `01_create_locked_notarization` example:

```bash
IOTA_NOTARIZATION_PKG_ID=0x... cargo run --release --example 01_create_locked_notarization
```

## Basic Examples

The following basic CRUD (Create, Read, Update, Delete) examples are available:

| Name                                                                                                                                  | Information                                                                       |
| :------------------------------------------------------------------------------------------------------------------------------------ | :-------------------------------------------------------------------------------- |
| [01_create_locked_notarization](https://github.com/iotaledger/notarization/tree/main/examples/01_create_locked_notarization.rs)       | Demonstrates how to create a locked notarization with delete locks.               |
| [02_create_dynamic_notarization](https://github.com/iotaledger/notarization/tree/main/examples/02_create_dynamic_notarization.rs)     | Demonstrates how to create dynamic notarizations with and without transfer locks. |
| [03_update_dynamic_notarization](https://github.com/iotaledger/notarization/tree/main/examples/03_update_dynamic_notarization.rs)     | Demonstrates that dynamic notarizations can be updated                            |
| [04_destroy_notarization](https://github.com/iotaledger/notarization/tree/main/examples/04_destroy_notarization.rs)                   | Demonstrates notarization destruction scenarios based on lock types.              |
| [05_update_state](https://github.com/iotaledger/notarization/tree/main/examples/05_update_state.rs)                                   | Demonstrates state updates on dynamic notarizations including binary data.        |
| [06_update_metadata](https://github.com/iotaledger/notarization/tree/main/examples/06_update_metadata.rs)                             | Demonstrates metadata updates and their behavior vs state updates.                |
| [07_transfer_dynamic_notarization](https://github.com/iotaledger/notarization/tree/main/examples/07_transfer_dynamic_notarization.rs) | Demonstrates transfer scenarios for different notarization types and lock states. |
| [08_access_read_only_methods](https://github.com/iotaledger/notarization/tree/main/examples/08_access_read_only_methods.rs)           | Comprehensive demonstration of all read-only inspection methods.                  |

## Notarization Types

### Dynamic Notarizations

- **Mutable**: State and metadata can be updated after creation
- **Transferable**: Can be transferred between addresses (unless transfer-locked)
- **Flexible locking**: Support optional transfer locks with time-based expiration
- **Version tracking**: State updates increment version counters

### Locked Notarizations

- **Immutable**: State and metadata cannot be changed after creation
- **Non-transferable**: Cannot be transferred to other addresses
- **Time-based destruction**: Can only be destroyed after delete lock expires (or with TimeLock::None)
- **Permanent record**: Provides immutable document notarization

## Lock Types

### Transfer Locks (Dynamic Notarizations Only)

- **None**: No transfer restrictions
- **UnlockAt(timestamp)**: Locked until specified timestamp
- **UntilDestroyed**: Locked until notarization is destroyed

### Delete Locks (Locked Notarizations Only)

- **None**: Can be destroyed immediately
- **UnlockAt(timestamp)**: Cannot be destroyed until timestamp
- **UntilDestroyed**: Cannot be destroyed (permanent)

## Key Concepts

### State Management

- **State Data**: The core content of the notarization (text or binary)
- **State Metadata**: Additional information about the state
- **Version Count**: Tracks number of state updates (dynamic only)

### Metadata Types

- **Immutable Description**: Set at creation, never changes
- **Updateable Metadata**: Can be modified on dynamic notarizations
- **Lock Metadata**: Automatically managed lock information

### Read-Only Operations

All notarizations support comprehensive inspection methods:

- Content and metadata retrieval
- Timestamp queries (creation, last change)
- Lock status checking
- Version tracking

## Example Scenarios

### Document Notarization Workflow

1. **Create** a notarization with initial content
2. **Update** state/metadata as document evolves (dynamic only)
3. **Transfer** ownership if needed (dynamic only, unless locked)
4. **Query** status and history at any time
5. **Destroy** when no longer needed (if locks permit)

### Compliance and Legal Use Cases

- **Locked notarizations** for regulatory compliance
- **Time-based locks** for audit trail requirements
- **Immutable records** for legal document preservation
- **Transfer controls** for ownership management

## Error Handling

The examples demonstrate proper error handling for common scenarios:

- Attempting to update locked notarizations
- Transferring locked notarizations
- Destroying locked notarizations before expiration
- Network and transaction failures

## Best Practices

1. **Choose the right type**: Use locked for immutable records, dynamic for evolving documents
2. **Set appropriate locks**: Consider regulatory and business requirements
3. **Handle errors gracefully**: Always check lock status before operations
4. **Use read-only methods**: Verify state before making changes
5. **Plan for lifecycle**: Consider destruction policies and transfer needs

## Security Considerations

- Notarizations are publicly readable on the blockchain
- Private keys control notarization operations
- Transfer locks prevent unauthorized ownership changes
- Delete locks ensure data retention requirements

For more detailed information about IOTA Notarization concepts and advanced usage, refer to the official IOTA documentation.
