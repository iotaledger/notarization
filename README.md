## About 

IOTA Notarization is a flexible toolkit that offers multiple methods - such as
Locked Notarization and Dynamic Notarization — to securely anchor, update, or
store data on the IOTA ledger.

--------------------------------------------------------------------
>
> :warning: This repository is currently work in process 
--------------------------------------------------------------------

The current implementation provides Rust libraries and Move smart contracts.

The main source of trust results from the provided Move smart contracts
which are published by the IOTA foundation as package here:
[TODO add Notarization package link to latest version here]().

The Rust libraries provide convenience functions to:
* interact with the Notarization Move smart contracts
* query state and meta data information from IOTA nodes and indexers

## Build

Prerequisites: Before you can build Move packages you need to install the IOTA CLI as being described
[here](https://docs.iota.org/developer/getting-started/install-iota)
.

To build the Move package contained in the `notarization` crate:

```bash
$ # In the folder `notarization-move`:
$
$ iota move build
```

## Run the test script *notarize.sh*

The provided experimental test script can be used to publish the Notarization package
and to experiment with the Notarization object, stored on the ledger.

### Prerequisites

Before you can use the testscript, please make sure you already prepared the following
* Prepare you local IOTA CLI environment as being described
  [here](https://docs.iota.org/developer/getting-started/connect)
* Activate the correct network environment (testnet, local,
  devnet) by executing: `iota client switch --env <network-alias>`
* Fetch some gas budget from the faucet
  by executing: `iota client faucet`
  
### Get Help

To read the provided help of the test script execute the following in the
folder `scripts`:

```bash
$ ./notarize.sh --help
  Usage: ./notarize.sh <command> [arguments]
  
  Commands:
    publish                                    Publish the contract
    create-dynamic <data> <metadata> <desc>    Create a dynamic notarization
    create-locked <data> <metadata> <desc> <update_lock> <delete_lock>
                                              Create a locked notarization
    update <id> <new_data> <new_metadata>     Update notarization state
    destroy <id>                              Destroy a notarization
  
  Examples:
    ./notarize.sh create-dynamic '[1,2,3]' 'Test data' 'My notarization'
    ./notarize.sh create-locked '[1,2,3]' 'Test data' 'Locked notarization' 3600 7200
    ./notarize.sh update 0x123...abc '[4,5,6]' 'Updated data'

```

### Publish the Move package

You can use the already published package on *testnet* (if it's still available)
without any further configuration.

The address of the used Notarization package is configured in the file
`scripts/notarize.sh`

```
# Package address of the notarization module (update after publishing)
PACKAGE_ADDRESS="0xf30e78de0bef4c76d1df30b5b8de20195ab46e2270f7a8378fc923b2c9675380"
```

If the already published package is no longer available or if you've changed the Move code,
you need to publish the package before you can test the Notarization functionality.

To publish the package:

```bash
$ # In the folder `scripts`
$
$ ./notarize.sh publish
Publishing contract from: /home/...
...
─────────────────────────────────────────────...
│ Transaction Data                           
├────────────────────────────────────────────...
...
...
│ Published Objects:
│  ┌──
│  │ PackageID: 0xf30e78de0bef4c76d1df30b5b8de20195ab46e2270f7a8378fc923b2c9675380
│  │ Version: 1
│  │ Digest: 64as4goqhoeYvPZ4Q9WcwjMFcaQQHmZctXRan7yj7gz7
│  │ Modules: lock_configuration, notarization, timelock_unlock_condition
│  └── 
...
```

Please search for the `PackageID` in the `Published Objects` section of the
`Object Changes` block written to the console log, and copy the value
of the `PackageID` into your system clipboard.

In the file `scripts/notarize.sh`, please replace the
preconfigured value for `PACKAGE_ADDRESS` with the copied value. 

### Creating Notarizations

The test script provides two options to create Notarization objects for
*Dynamic-* and *Locked-Notarization* methods:

* `./notarize.sh create-dynamic ...`
* `./notarize.sh create-locked ...`

This is described in the following sections in more detail.

#### Create a Dynamic Notarization

To create a *Dynamic Notarization* execute the following in the
folder `scripts`:

```bash
$ ./notarize.sh create-dynamic '[1,2,3]' 'Initial version of state data' 'Some immutable Description'
...
...
Transaction Digest: 3DvXWMK4ZHfnFT...
╭──────────────────────────────────────────────...
│ Transaction Data                             ...
├──────────────────────────────────────────────...
...
...
╭──────────────────────────────────────────────...
│ Object Changes                               
├──────────────────────────────────────────────...
│ Created Objects:                             
│  ┌──                                         
│  │ ObjectID: 0x354744923ecae0957822be4941d02c4de6e29da7012aaba3eb3072547595f66f
│  │ Sender: 0xd84350eefac208723547c6d1c16a69fc...
│  │ Owner: Account Address ( 0xd84350eefac2087...
│  │ ObjectType: 0xf30e78de0bef4c76d1df30b5b8de...
│  │ Version: 151166465                        
│  │ Digest: HfteRwmz6ht3PUCFynGs1NEU6GHobDFn83...
│  └── 
...
...
```
Please search the `ObjectID` in the `Created Objects` section of the
`Object Changes` block written to the console, and copy/note it for later use.

The created Notarization object can be investigated using the iota explorer
using the copied `ObjectID`.

For example to investigate the above created object using the testnet
explorer please open the following link:
https://explorer.rebased.iota.org/object/0x354744923ecae0957822be4941d02c4de6e29da7012aaba3eb3072547595f66f?network=testnet

#### Create a Locked Notarization

To create a *Locked Notarization* being update-locked until January 1st 2035 12am CET
and delete-locked ca. 3:20 minutes later,
execute the following in the folder `scripts`:

```bash
$ ./notarize.sh create-locked '[1,2,3]' 'Test data' 'Locked notarization' 2051218800 2051219000
...
...
Transaction Digest: DQL3dEjpeGzzaUS...
╭──────────────────────────────────────────────...
│ Transaction Data                             ...
├──────────────────────────────────────────────...
...
...
╭──────────────────────────────────────────────...
│ Object Changes                               
├──────────────────────────────────────────────...
│ Created Objects:                             
│  ┌──                                         
│  │ ObjectID: 0xa1f4688fb8688dc35674a3c3e0ec332be32dcee5abcf131f833587aa9e6ff7fa
│  │ Sender: 0xd84350eefac208723547c6d1c16a69fc...
│  │ Owner: Account Address ( 0xd84350eefac2087...
│  │ ObjectType: 0xf30e78de0bef4c76d1df30b5b8de...
│  │ Version: 150153289                        
│  │ Digest: EqLELTLt9Ww2Wko516fToCaoYY7BNmffXx...
│  └── 
...
...
```

As being described in the [Create a Dynamic Notarization](#create-a-dynamic-notarization) section 
you can investigate the created object using the testnet
explorer and the `ObjectID` logged to the console.
 
To investigate the above created object please use the following link:
https://explorer.rebased.iota.org/object/0xa1f4688fb8688dc35674a3c3e0ec332be32dcee5abcf131f833587aa9e6ff7fa?network=testnet

### Updating Notarizations

To update Notarizations the test script provides the `update` option.

*Dynamic Notarizations* can be locked at any time while *Locked Notarizations*
can only be updated if the *update_lock* has expired.

#### Update a Dynamic Notarization

Please execute in the folder `scripts`:
```bash
$ ./notarize.sh update 0x354744923ecae0957822be4941d02c4de6e29da7012aaba3eb3072547595f66f '[4,5,6]' 'Data updated first time'
Updating notarization state...
Notarization ID: 0x354744923ecae0957822be4941d02c4de6e29da7012aaba3eb3072547595f66f
New data: [4,5,6]
New metadata: Data updated first time
Transaction Digest: GNQxcRavA2PZfzgE7cFhGfCmxWrs3VwAvfQd5xTedY7Q
...
──────────────────────────────────────────────────────────────────────────────────...
│ Transaction Block Events                                                        
├─────────────────────────────────────────────────────────────────────────────────...
│  ┌──                                                                            
│  │ EventID: GNQxcRavA2PZfzgE7cFhGfCmxWrs3VwAvfQd5xTedY7Q:0                      
│  │ PackageID: 0xf30e78de0bef4c76d1df30b5b8de20195ab46e2270f7a8378fc923b2c9675380
│  │ Transaction Module: notarization                                             
│  │ Sender: 0xd84350eefac208723547c6d1c16a69fc53b561ecd4c613b5afd2661f96c5e01c   
│  │ EventType: 0xf30e78de0bef4c76d1df30b5b8de20195ab46e2270f7a8378fc923b2c9675380::notarization::NotarizationUpdated
│  │ ParsedJSON:                                                                                   
│  │   ┌─────────────────────┬────────────────────────────────────────────────────────────────────┐
│  │   │ notarization_obj_id │ 0x354744923ecae0957822be4941d02c4de6e29da7012aaba3eb3072547595f66f │
│  │   ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
│  │   │ state_version_count │ 1                                                                  │
│  │   └─────────────────────┴────────────────────────────────────────────────────────────────────┘
│  └──                                                                                             
╰──────────────────────────────────────────────────────────────────────────────────────────────────...
╭──────────────────────────────────────────────...
│ Object Changes                               
├──────────────────────────────────────────────...
│ Mutated Objects:
...
...
│  ┌──                                         
│  │ ObjectID: 0x354744923ecae0957822be4941d02c4de6e29da7012aaba3eb3072547595f66f
│  │ Sender: 0xd84350eefac208723547c6d1c16a69fc...
│  │ Owner: Account Address ( 0xd84350eefac2087...
│  │ ObjectType: 0xf30e78de0bef4c76d1df30b5b8de...
│  │ Version: 151184440                        
│  │ Digest: Ay48t5in2zLey2uVz8TM2pVnUvzeVJMaDP...
│  └── 
...
```
To investigate the above created object please use the following link:
https://explorer.rebased.iota.org/object/0x354744923ecae0957822be4941d02c4de6e29da7012aaba3eb3072547595f66f?network=testnet

#### Update a Locked Notarization

Please execute in the folder `scripts`:

```bash
$ ./notarize.sh update 0xa1f4688fb8688dc35674a3c3e0ec332be32dcee5abcf131f833587aa9e6ff7fa '[4,5,6]' 'Data updated first time'
Updating notarization state...
Notarization ID: 0xa1f4688fb8688dc35674a3c3e0ec332be32dcee5abcf131f833587aa9e6ff7fa
New data: [4,5,6]
New metadata: Data updated first time
Error executing transaction '4AK61oFuAzbyrthwxYJtQuDHPJXsCJFL6gX7ak3XUjgb': 3rd command aborted within function '0xf30e78de0bef4c76d1df30b5b8de20195ab46e2270f7a8378fc923b2c9675380::notarization::update_state' at instruction 10 with code 0
```

As the example Notarization object, already published in the examples above, is
update-locked until January 1st 2035 the update is not possible for a while,
which is indicated by the smart contract code with an error: 
`<PackageID>::notarization::update_state' at instruction 10 with code 0` 
which can be translated to an
`iota_notarization::notarization::EUpdateWhileLocked` error.

Feel free to create your own updatable Notarization object instance on the ledger
and try updating it.

## Docs

TBD

## Issues

See the [open issues](https://github.com/iotaledger/notarization/issues) for a full list of proposed features (and known issues).

## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

Distributed under the Apache License. See `LICENSE` for more information.
