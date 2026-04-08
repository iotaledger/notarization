const { _ } = Cypress;

describe(
    "Test Examples",
    () => {
        const examples = [
            "01_create_audit_trail",
            "02_add_and_read_records",
            "03_update_metadata",
            "04_configure_locking",
            "05_manage_access",
            "06_delete_records",
            "07_access_read_only_methods",
            "08_delete_audit_trail",
            "09_tagged_records",
            "10_capability_constraints",
            "11_manage_record_tags",
            "01_customs_clearance",
            "02_clinical_trial",
        ];

        _.each(examples, (example) => {
            it(example, () => {
                cy.visit("/", {
                    onBeforeLoad(win) {
                        cy.stub(win.console, "log").as("consoleLog");
                    },
                });
                cy.get("@consoleLog").should("be.calledWith", "init");
                cy.window().then(win => win.runTest(example));
                cy.get("@consoleLog").should("be.calledWith", "success");
            });
        });
    },
);