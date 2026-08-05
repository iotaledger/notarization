// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const lockPath = resolve(packageRoot, "grpc/iota-schema.lock.json");
const imagePath = resolve(packageRoot, "grpc/iota-ledger.binpb");
const temporaryImagePath = `${imagePath}.tmp`;
const lock = JSON.parse(await readFile(lockPath, "utf8"));
const revision = process.argv[2] ?? lock.revision;

if (!/^[0-9a-f]{40}$/.test(revision)) {
  throw new Error("IOTA SDK revision must be a complete 40-character lowercase Git commit");
}

if (lock.repository !== "https://github.com/iotaledger/iota-rust-sdk") {
  throw new Error(`refusing to download schema from unapproved repository: ${lock.repository}`);
}

const archive = `${lock.repository}/archive/${revision}.tar.gz`;
const input = `${archive}#strip_components=1,subdir=${lock.protoRoot}`;
const args = ["build", input, "--timeout", "60s", "--output", temporaryImagePath];

for (const entrypoint of lock.entrypoints) {
  args.push("--path", entrypoint);
}

await rm(temporaryImagePath, { force: true });

try {
  await run("buf", args);

  const image = await readFile(temporaryImagePath);
  const imageSha256 = `sha256:${createHash("sha256").update(image).digest("hex")}`;

  await rename(temporaryImagePath, imagePath);
  await writeFile(
    lockPath,
    `${JSON.stringify({ ...lock, revision, imageSha256 }, null, 2)}\n`,
  );

  console.log(`Updated IOTA gRPC schema to ${revision}`);
  console.log(`Buf image: ${imageSha256}`);
} finally {
  await rm(temporaryImagePath, { force: true });
}

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

