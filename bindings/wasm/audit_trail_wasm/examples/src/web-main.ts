// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

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

export async function main(example?: string) {
    const argument = example ?? new URLSearchParams(window.location.search).get("example")?.toLowerCase();
    if (!argument) {
        throw new Error("Please specify an example name, e.g. '01_create_audit_trail'");
    }

    switch (argument) {
        case "01_create_audit_trail":
            return createAuditTrail();
        case "02_add_and_read_records":
            return addAndReadRecords();
        case "03_update_metadata":
            return updateMetadata();
        case "04_configure_locking":
            return configureLocking();
        case "05_manage_access":
            return manageAccess();
        case "06_delete_records":
            return deleteRecords();
        case "07_access_read_only_methods":
            return accessReadOnlyMethods();
        case "08_delete_audit_trail":
            return deleteAuditTrail();
        case "09_tagged_records":
            return taggedRecords();
        case "10_capability_constraints":
            return capabilityConstraints();
        case "11_manage_record_tags":
            return manageRecordTags();
        case "01_customs_clearance":
            return customsClearance();
        case "02_clinical_trial":
            return clinicalTrial();
        default:
            throw new Error(`Unknown example name: '${argument}'`);
    }
}