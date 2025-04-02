# IOTA Notarization

The Notarization Rust library provides a `NotarizationBuilder` that can be used to create Notarization objects on
the IOTA ledger or to use an already existing Notarization object. The NotarizationBuilder returns a Notarization struct
instance, which is mapped to the Notarization object on the ledger and can be used to interact with the object.

Following Notarization methods are currently provided:
* Dynamic Notarization
* Locked Notarization

These Notarization methods are implemented using a single Notarization Move object, stored on the IOTA Ledger.
The Method specific behavior is achieved via configuration of this object.

**TODO: Link to full docs of config parameters**

To minimize the need for config settings, the Notarization methods reduce the number of available configuration
parameters while using method specific fixed settings for several parameters, resulting in the typical method
specific behaviour. Here, Notarization methods can be seen as a prepared configuration sets to facilitate
Notarization usage for often needed use cases.

Here is an overview of the most important configuration parameters for each of these methods:

| Method  | Locking exists  | delete_lock*     | update_lock             | transfer_lock           |
|---------|-----------------|------------------|-------------------------|-------------------------|
| Dynamic | Optional [conf] | None [static]    | None [static]           | Optional [conf]         |
| Locked  | Yes [static]    | Optional* [conf] | UntilDestroyed [static] | UntilDestroyed [static] |


Explanation of terms and symbols for the table above:
* [conf]: Configurable parameter.
* [static]: Fixed or static parameter.
* Optional:
  * Locks: The lock can be set to UnlockAt or UntilDestroyed.
  * Locking exists: If no locking is used, there will be no [`LockMetadata`] stored with Notarization object.
    otherwise [`LockMetadata`] will be created automatically. 
* *: delete_lock can not be set to UntilDestroyed.

## Process Flows

The following workflows demonstrate how NotarizationBuilder and Notarization instances can be used to create, update and
destroy Notarization objects on the ledger.

### Dynamic Notarizations

A Dynamic Notarization is created on the ledger using the NotarizationBuilder::create_dynamic() function. To create a Dynamic Notarization, the following initial arguments need to be specified using the NotarizationBuilder setter functions (The used terms can be found in the Epic Glossary):

* Initial State consisting of Stored Data and State Metadata that will be used to define the first version of the Notarization state.
* Optional Immutable Description
* Optional Updateable Metadata
* A boolean indicator if the Notarization shall be transferable

After the Notarization has been created, it can be updated using the Notarization::update_state() function and can be destroyed using Notarization::destroy().

#### Creating a new Dynamic Notarization on the Ledger

The following sequence diagram explains the interaction between the involved technical components and the Prover when a Dynamic Notarization is created on the ledger:

````mermaid
sequenceDiagram
    actor Prover
    participant Lib as Rust-Library
    participant Move as Move-SC
    participant Net as Iota-Network
    Prover->>+Lib: fn NotarizationBuilder::new()
    Lib->>-Prover: NotarizationBuilder
    Prover->>Lib: fn NotarizationBuilder::iota_client()
    Prover->>Lib: fn NotarizationBuilder::signer()
    Prover->>Lib: fn NotarizationBuilder::immutable_description()
    Prover->>Lib: fn NotarizationBuilder::initial_state()
    Note right of Prover: State = Binary Data + Metadata
    Prover->>+Lib: fn NotarizationBuilder::create_dynamic()
    Note right of Prover: Alternatively fn build_ptb() <br> can be used to only return the <br> programmable transaction block
    Lib->>+Move: notarization::new_state_from_vector()
    Move->>-Lib: Notarization State
    Lib->>+Move: dynamic_notarization::create()
    Move->>Net: transfer::transfer(notarization, sender)
    Note right of Move: This is omitted if fn build_ptb() is used
    Move->>-Lib: TX Response
    Lib->>-Prover: Notarization + TX Response
````
#### Fetching state data from a Notarization already existing on the ledger
Fetching state data from a Notarization already existing on the ledger

The following sequence diagram explains the component interaction for Verifiers (or other parties) fetching the Latest State:

````mermaid
sequenceDiagram
    actor Verifier
    participant Lib as Rust-Library
    participant Move as Move-SC
    participant Net as Iota-Network
    Verifier->>+Lib: fn NotarizationBuilder::new()
    Lib->>-Verifier: NotarizationBuilder
    Verifier->>Lib: fn NotarizationBuilder::iota_client()
    Verifier->>Lib: fn NotarizationBuilder::object_id()
    Verifier->>+Lib: fn NotarizationBuilder::finish()
    Lib->>-Verifier: Notarization
    Verifier->>+Lib: fn Notarization::get_state()
    Lib-->>Net: RPC Calls
    Net-->>Lib: State Data
    Lib->>-Verifier: State
````
#### Updating state data of a Notarization already existing on the ledger

The following sequence diagram shows the component interaction in case a Prover wants to update the  Latest State of a Notarization:

````mermaid
sequenceDiagram
    actor Prover
    participant Lib as Rust-Library
    participant Move as Move-SC
    participant Net as Iota-Network
    Prover->>+Lib: fn NotarizationBuilder::new()
    Lib->>-Prover: NotarizationBuilder
    Prover->>Lib: fn NotarizationBuilder::iota_client()  
    Prover->>Lib: fn NotarizationBuilder::object_id()
    Prover->>Lib: fn NotarizationBuilder::signer()
    Note right of Prover: Needed for write access      
    Prover->>+Lib: fn NotarizationBuilder::finish()
    Lib->>-Prover: Notarization
    Prover->>+Lib: fn Notarization::update_state()
    Note right of Prover: Alternatively fn build_update_state_ptb() <br> can be used to only return the <br> programmable transaction block  
    Lib->>+Move: notarization::new_state_from_vector()
    Move->>-Lib: Notarization State
    Lib->>+Move: notarization::update_state()
    Move->>Net: updates object fields on ledger
    Note right of Move: Object update and emitting the <br> NotarizationUpdated Event is omitted <br> if fn build_update_state_ptb() is used
    Move->>Net: event::emit(NotarizationUpdated)
    Move->>-Lib: TX Response
    Lib->>-Prover: TX Response
````

### Locked Notarizations

In general Locked Notarizations are handled similar to Dynamic Notarizations. A Locked Notarization is created on the ledger using the NotarizationBuilder::create_locked() function.

To create a Locked Notarization the following arguments need to be specified using the NotarizationBuilder setter functions:

* all arguments needed to create a Dynamic Notarization
* Optional Delete Timelock

After the Locked Notarization has been created - by design - the Latest State can not bee updated anymore.

The lifecycle of a Locked Notarization can be described as:

* Create a Notarization object using the NotarizationBuilder
* If a Delete Timelock has been used, wait at least until the time-lock has expired
* Destroy the Notarization object

As the Latest State of a Locked Notarization can not bee updated the lifecycle doesn’t include any update processes.