/// <reference types="vite/client" />
declare const IOTA_AUDIT_TRAIL_PKG_ID: string;
declare const IOTA_TF_COMPONENTS_PKG_ID: string;
declare const NETWORK_NAME_FAUCET: string;
declare const ENV_NETWORK_URL: string;
declare const runTest: (example: string) => Promise<void>;

declare global {
    var runTest: (example: string) => Promise<void>;
}

export {};
