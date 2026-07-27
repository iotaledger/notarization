// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const lock = JSON.parse(
  await readFile(resolve(packageRoot, "grpc/iota-schema.lock.json"), "utf8"),
);
const image = await readFile(resolve(packageRoot, "grpc/iota-ledger.binpb"));
const actualDigest = `sha256:${createHash("sha256").update(image).digest("hex")}`;

if (lock.imageSha256 !== actualDigest) {
  throw new Error(
    `IOTA schema image digest mismatch: expected ${lock.imageSha256}, received ${actualDigest}`,
  );
}

await run("buf", ["generate", "--template", "grpc/buf.gen.yaml"]);

function run(command, args) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, {
      cwd: packageRoot,
      env: {
        ...process.env,
        BUF_CACHE_DIR: resolve(packageRoot, ".cache/buf"),
      },
      stdio: "inherit",
      shell: false,
    });

    child.once("error", (error) => {
      reject(new Error(`failed to start ${command}: ${error.message}`, { cause: error }));
    });
    child.once("exit", (code, signal) => {
      if (code === 0) {
        resolvePromise();
      } else {
        reject(new Error(`${command} failed with ${signal ? `signal ${signal}` : `exit code ${code}`}`));
      }
    });
  });
}

