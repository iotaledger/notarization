// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, it } from "mocha";

import { createAuditTrail } from "./01_create_audit_trail";
import { addAndReadRecords } from "./02_add_and_read_records";
import { updateMetadata } from "./03_update_metadata";
import { configureLocking } from "./04_configure_locking";
import { manageAccess } from "./05_manage_access";
import { deleteRecords } from "./06_delete_records";
import { accessReadOnlyMethods } from "./07_access_read_only_methods";
import { deleteAuditTrail } from "./08_delete_audit_trail";
import { taggedRecords } from "./advanced/09_tagged_records";
import { capabilityConstraints } from "./advanced/10_capability_constraints";
import { manageRecordTags } from "./advanced/11_manage_record_tags";
import { customsClearance } from "./real-world/01_customs_clearance";
import { clinicalTrial } from "./real-world/02_clinical_trial";

describe("Audit trail wasm node examples", function() {
    afterEach(() => {
        console.log("\n----------------------------------------------------\n");
    });

    it("creates a trail", async () => {
        await createAuditTrail();
    });
    it("adds and reads records", async () => {
        await addAndReadRecords();
    });
    it("updates metadata", async () => {
        await updateMetadata();
    });
    it("configures locking", async () => {
        await configureLocking();
    });
    it("manages access", async () => {
        await manageAccess();
    });
    it("deletes records", async () => {
        await deleteRecords();
    });
    it("accesses read-only methods", async () => {
        await accessReadOnlyMethods();
    });
    it("deletes an audit trail", async () => {
        await deleteAuditTrail();
    });
    it("uses tagged records", async () => {
        await taggedRecords();
    });
    it("constrains capabilities", async () => {
        await capabilityConstraints();
    });
    it("manages record tags", async () => {
        await manageRecordTags();
    });
    it("runs customs clearance example", async () => {
        await customsClearance();
    });
    it("runs clinical trial example", async () => {
        await clinicalTrial();
    });
});
