const { _ } = Cypress;

describe(
    "Test Examples",
    () => {
        const examples = [
            "01_create_locked",
            "02_create_dynamic",
            "03_update_dynamic",
            "04_destroy_notarization",
            "05_update_state",
            "06_update_metadata",
            "07_transfer_notarization",
            "08_access_read_only_methods",
            "01_real_world_iot_weather_station",
            "02_real_world_legal_contract",
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
