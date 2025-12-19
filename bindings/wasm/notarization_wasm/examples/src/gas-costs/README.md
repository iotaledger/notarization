# Gas Cost Estimation Example for Notarization

This folder contains an example to estimate the gas cost for Notarization object creation , update anf destroy operations.

It can be run like any other example.

The log output of the example is optimized to evaluate variables and constants needed to calculate gas cost as being
described in the following sections.

## Creating Notarizations

The cost for creating a Notarization object can roughly be calculated by the following equation:

            `TotalCost` = `FlexDataSize` * `FlexDataByteCost` + `MinimumStorageCost` + `ComputationCost`

            `TotalCost` = F [Byte] * 0.0000076 [IOTA/Byte] + 0.00295 [IOTA] + 0.001 [IOTA]

Where:

| Parameter            | Description                                                                                                                                                                                                                          |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `FlexDataSize`       | Sum of the byte sizes of State Data, State Metadata, Updatable Metadata and Immutable Metadata. The value must be reduced by 1 as the `MinimumStorageCost` uses 1 byte of State Data.                                                |
| `FlexDataByteCost`   | A constant value of 0.0000076 IOTA/Byte <br> This value denotes (`StorageCost` - `MinimumStorageCost`) divided by `FlexDataSize`.                                                                                                    |
| `MinimumStorageCost` | A constant value of 0.00295 IOTA. <br> This value denotes the `StorageCost` for a Notarization with 1 Byte of `FlexDataSize` meaning a Notarization with 1 Byte of State Data, no meta data and no optional locks.                   |
| `ComputationCost`    | A constant value of 0.001 IOTA. <br> Given the Gas Price is 1000 nano, the `ComputationCost` will always be 0.001 IOTA as creating Notarizations always consume 1000 Computation Units.                                              |
| `TotalCost`          | The amount of IOTA that would need to be paid for gas when Storage Rebate is not taken into account. The real gas cost will be lower, due to Storage Rebate, which is usually -0.0009804 IOTA when a Notarization object is created. |

Examples:

| `FlexDataSize` | `TotalCost` (Storage Rebate not taken into account) |
| -------------- | --------------------------------------------------- |
| 10             | 0.004026 IOTA                                       |
| 100            | 0.00471 IOTA                                        |
| 1000           | 0.01155 IOTA                                        |

## Updating Dynamic Notarizations

The `TotalCost` for updating a Dynamic Notarization can roughly be calculated using the same equation used for creating
Notarization objects (see above).

The value for `FlexDataByteCost` should be set to 0.00000769 IOTA/Byte.

If the new Notarization State results in the same `FlexDataSize` as the overwritten old Notarization State, the Storage
Rebate will compensate the Storage Cost so that the real gas cost to be paid will be more or less the Computation Cost,
which is always 0.001 IOTA (presumed the Gas Price is 1000 nano).

## Destroying a Notarization

The `TotalCost` for destroying a Notarization is the Computation Cost which is 0.001 IOTA (presumed the Gas Price is 1000 nano).

Due to the Storage Rebate, which depends on the size of the stored Notarization object, the real gas cost to be paid will often be negative.

The Storage Rebate can roughly be calculated using the below equation. See above for more details about the used variables and constants.
