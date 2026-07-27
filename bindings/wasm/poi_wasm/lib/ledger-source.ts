// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import type { Status } from "./grpc/generated/google/rpc/status_pb.js";
import type { ExecutedTransaction } from "./grpc/generated/iota/grpc/v1/transaction_pb.js";
import {
  createIotaGrpcClient,
  type IotaGrpcClient,
  type IotaGrpcClientOptions,
} from "./client.js";
import type {
  CheckpointEvidence,
  Committee,
  LedgerSource as LedgerSourceContract,
  TransactionEvidence,
} from "./source-types.js";

const CHAIN_IDENTIFIER_FIELDS = ["chain_id"];
const COMMITTEE_FIELDS = ["committee"];
const OBJECT_PROOF_FIELDS = ["bcs"];
const TRANSACTION_PROOF_FIELDS = [
  "transaction.bcs",
  "signatures",
  "effects.bcs",
  "events.digest",
  "events.events.bcs",
  "checkpoint",
];
const CHECKPOINT_PROOF_FIELDS = [
  "checkpoint.summary.bcs",
  "checkpoint.signature",
  "checkpoint.contents.bcs",
];

/**
 * Internal implementation of the ledger reads required by Proof of Inclusion.
 *
 * The generated client owns gRPC and protobuf. This class narrows its responses
 * to BCS evidence that can cross the JavaScript/WASM boundary.
 */
export class LedgerSource implements LedgerSourceContract {
  readonly #client: IotaGrpcClient;

  public constructor(endpoint: string, options: IotaGrpcClientOptions = {}) {
    this.#client = createIotaGrpcClient(endpoint, options);
  }

  public async chainIdentifier(): Promise<Uint8Array> {
    const response = await this.#client.getServiceInfo({
      readMask: { paths: CHAIN_IDENTIFIER_FIELDS },
    });

    return response.chainId!.digest!;
  }

  public async transaction(
    digest: Uint8Array,
  ): Promise<TransactionEvidence | undefined> {
    let transaction: ExecutedTransaction | undefined;

    for await (const response of this.#client.getTransactions({
      requests: {
        requests: [{ digest: { digest } }],
      },
      readMask: { paths: TRANSACTION_PROOF_FIELDS },
    })) {
      for (const result of response.transactionResults) {
        if (result.result.case === "error") {
          throw statusError("getTransactions", result.result.value);
        }

        if (result.result.case !== "executedTransaction") {
          throw new Error(
            "getTransactions returned a result without a transaction or error",
          );
        }

        if (transaction) {
          throw new Error(
            "getTransactions returned more than one transaction for one digest",
          );
        }

        transaction = result.result.value;
      }
    }

    if (!transaction) {
      return undefined;
    }

    const signatures = transaction.signatures!;
    const transactionEvents = transaction.events?.events?.events;

    return {
      transactionBcs: transaction.transaction!.bcs!.data!,
      signaturesBcs: signatures.signatures.map(
        (signature) => signature.bcs!.data!,
      ),
      effectsBcs: transaction.effects!.bcs!.data!,
      eventsBcs: transactionEvents?.map((event) => event.bcs!.data!),
      checkpointSequenceNumber: transaction.checkpoint!,
    };
  }

  public async object(
    objectId: Uint8Array,
    version?: bigint,
  ): Promise<Uint8Array | undefined> {
    let objectBcs: Uint8Array | undefined;

    for await (const response of this.#client.getObjects({
      requests: {
        requests: [
          {
            objectRef: {
              objectId: { objectId },
              version,
            },
          },
        ],
      },
      readMask: { paths: OBJECT_PROOF_FIELDS },
    })) {
      for (const result of response.objects) {
        if (result.result.case === "error") {
          throw statusError("getObjects", result.result.value);
        }

        if (result.result.case !== "object") {
          throw new Error(
            "getObjects returned a result without an object or error",
          );
        }

        if (objectBcs) {
          throw new Error("getObjects returned more than one object for one ID");
        }

        objectBcs = result.result.value.bcs!.data!;
      }
    }

    return objectBcs;
  }

  public async checkpoint(
    sequenceNumber: bigint,
  ): Promise<CheckpointEvidence> {
    let checkpoint: CheckpointEvidence | undefined;
    let reachedEnd = false;

    for await (const response of this.#client.getCheckpoint({
      checkpointId: {
        case: "sequenceNumber",
        value: sequenceNumber,
      },
      readMask: { paths: CHECKPOINT_PROOF_FIELDS },
    })) {
      if (response.payload.case === "checkpoint") {
        if (checkpoint) {
          throw new Error(
            "getCheckpoint returned more than one checkpoint for one sequence number",
          );
        }

        const value = response.payload.value;

        if (
          value.sequenceNumber !== undefined &&
          value.sequenceNumber !== sequenceNumber
        ) {
          throw new Error(
            `getCheckpoint returned sequence number ${value.sequenceNumber}, expected ${sequenceNumber}`,
          );
        }

        checkpoint = {
          summaryBcs: value.summary!.bcs!.data!,
          signatureBcs: value.signature!.bcs!.data!,
          contentsBcs: value.contents!.bcs!.data!,
        };
      } else if (response.payload.case === "endMarker") {
        const returnedSequenceNumber = response.payload.value.sequenceNumber;

        if (
          returnedSequenceNumber !== undefined &&
          returnedSequenceNumber !== sequenceNumber
        ) {
          throw new Error(
            `getCheckpoint ended at sequence number ${returnedSequenceNumber}, expected ${sequenceNumber}`,
          );
        }

        reachedEnd = true;
      }
    }

    if (!checkpoint) {
      throw new Error(
        `getCheckpoint returned no checkpoint for sequence number ${sequenceNumber}`,
      );
    }

    if (!reachedEnd) {
      throw new Error(
        `getCheckpoint did not finish sequence number ${sequenceNumber}`,
      );
    }

    return checkpoint;
  }

  public async committee(epoch: bigint): Promise<Committee> {
    const response = await this.#client.getEpoch({
      epoch,
      readMask: { paths: COMMITTEE_FIELDS },
    });
    const committee = response.epoch!.committee!;

    return {
      members: committee.members!.members.map((member) => ({
        publicKey: member.publicKey!,
        weight: member.weight!,
      })),
    };
  }
}

function statusError(method: string, status: Status): Error {
  const details = status.message ? `: ${status.message}` : "";

  return new Error(`${method} failed with gRPC status ${status.code}${details}`);
}
