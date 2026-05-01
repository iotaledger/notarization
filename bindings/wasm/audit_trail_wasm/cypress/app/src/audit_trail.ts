import url from "@iota/audit-trail/web/audit_trail_wasm_bg.wasm?url";

import { init } from "@iota/audit-trail/web";
import { main } from "../../../examples/dist/web/web-main";

export const runTest = async (example: string) => {
    try {
        await main(example);
        console.log("success");
    } catch (error) {
        throw error;
    }
};

init(url)
    .then(() => {
        console.log("init");
    });
