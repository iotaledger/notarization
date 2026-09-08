# IOTA Notarization Toolkit Wasm Packages

This folder contains the Notarization Toolkit Wasm Packages. Each Package uses `wasm-bindgen` to expose Rust types and
functions to JavaScript and TypeScript runtimes.

The `build` folder provides the shared scripts needed to build the Packages.

The following Packages are available:

- `notarization_wasm`<br>
  Public surface of notarization-rs exported to JS/TypeScript
- `audit_trail_wasm`<br>
  Public surface of audit-trail-rs exported to JS/TypeScript
- `poi_wasm`<br>
  Proof of Inclusion client and public surface of poi-rs exported to JS/TypeScript

## Building a Package

See each Package README for its build instructions.

## Build process in general

Each Package has its own folder containing the following files and subfolders:

- `tsconfig` files for the `nodejs` and `web` runtimes
- The `package.json` file
- `lib` folder<br>
  Contains TS files used for wasm-bindings
  - Contains `tsconfig` files for the `nodejs` and `web` runtimes with additional TS compiler configurations
- `node` folder<br>
  Distribution folder for the Node.js runtime
- `web` folder<br>
  Distribution folder for the `web` runtime
- `src` folder<br>
  Rust code of the Package
- `tests` folder<br>
  Test code
- `examples` folder<br>
  Example code

The scripts in each Package's `package.json` file define its build process.
The build process for the Node.js and web runtimes consists of the following steps:

- Cargo build of the crate with target `wasm32-unknown-unknown`
- `wasm-bindgen` CLI call, generating `___.js` and `___.d.ts` files in the Package distribution folder (`node` or
  `web`)
- execute the `build/node` or `build/web` build script (see below)
- TypeScript compiler call (`tsc`)<br>
  Converts the TS files in the `lib` folder into JS files.
  JS files are written into the Package distribution folder.
  The distribution folder is configured
  in the applied `tsconfig` file located in the Package's `lib` folder.
- execute the `build/replace_paths` build script (see below)

## Build scripts contained in the `build` folder

### node.js

Used by the `bundle:nodejs` script in the Package's `package.json` file.

Process steps:

- Add a [node-fetch polyfill](https://github.com/seanmonstar/reqwest/issues/910)
  at the top of the Package's main JS file
- Generate a `package.json` file derived from the Package's original `package.json`
  (done by `utils/generatePackage.js`)

### web.js

Used by the `bundle:web` script in the Package's `package.json` file.

Process steps:

- In the Package's main JS file:
  - Comment out a webpack workaround by commenting out all occurrences of<br>
    `input = new URL(<SOME_CAPTURED_REGEX_GROUP>, import.meta.url);`
  - Create an initialization function that imports the Package's Wasm file.
- In the typescript source map file `<ARTIFACT_NAME>.d.ts`:
  - Adds the declaration of the above created init function to the typescript source map file
- Generate a `package.json` file derived from the Package's original `package.json`
  (done by `utils/generatePackage.js`)

### replace_paths.js

Processes all JS and TS files previously created in the Package distribution folder
by wasm-bindgen and the TS compiler (tsc) call.

For each file, it replaces aliases defined in the
[compilerOptions.paths](https://www.typescriptlang.org/docs/handbook/modules/reference.html#paths)
configuration of a specific
tsconfig file by the last entry of the aliases path list (only 1 or 2 paths supported).

It is used by the following run tasks for the following tsconfig files and distribution folders:

| run task             | tsconfig file                  | distribution folder |
| -------------------- | ------------------------------ | ------------------- |
| `bundle:nodejs`      | `./lib/tsconfig.json`          | `node`              |
| `bundle:web`         | `./lib/tsconfig.web.json`      | `web`               |
| `build:examples:web` | `./examples/tsconfig.web.json` | `./examples/dist`   |

## Documentation Style Guide for generated TSDoc/JSDoc

The [DOC-STYLEGUIDE.md](./DOC-STYLEGUIDE.md) defines the documentation rules for Rust types compiled into
JavaScript/TypeScript types with `wasm-bindgen`.

These rules are obligatory for developers and AI agents.
