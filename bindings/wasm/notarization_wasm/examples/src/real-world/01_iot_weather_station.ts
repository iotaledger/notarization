// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # IoT Weather Station Example - Dynamic Notarization
 *
 * This example demonstrates how to use notarization fields for a real-world IoT weather station
 * that continuously reports temperature and humidity readings.
 *
 * ## Field Usage Strategy:
 *
 * - **state.data**: Current sensor readings (temperature, humidity, pressure)
 * - **state.metadata**: Measurement context (location, timestamp, sensor info)
 * - **immutable_description**: Weather station device information (model, sensors)
 * - **updatable_metadata**: Device status and operational info (battery, signal, maintenance)
 *
 * This showcases how dynamic notarizations can evolve over time while maintaining
 * a clear separation of concerns between different types of data.
 */

import { OnChainNotarization, State } from "@iota/notarization/node";
import { getFundedClient } from "../util";

interface WeatherReading {
    temperature_celsius: number;
    humidity_percent: number;
    pressure_hpa: number;
    timestamp: number;
}

interface WeatherUpdate {
    temp: number;
    humidity: number;
    pressure: number;
    description: string;
}

/** Demonstrate IoT Weather Station using Dynamic Notarization */
export async function iotWeatherStation(): Promise<void> {
    console.log("🌡️ IoT Weather Station - Dynamic Notarization Example");
    console.log("=====================================================\n");

    const notarizationClient = await getFundedClient();

    // Get current timestamp for realistic data
    const now = Math.floor(Date.now() / 1000);

    console.log("📡 Creating weather station notarization...");

    // Create initial weather reading
    const initialReading: WeatherReading = {
        temperature_celsius: 16.2,
        humidity_percent: 65.0,
        pressure_hpa: 1013.25,
        timestamp: now,
    };

    const initialMetadata = `Location: Hamburg, Germany | Coordinates: 53.5488°N, 9.9872°E | Recorded: ${formatTimestamp(now)
        }`;

    // Create dynamic notarization for weather station
    const weatherNotarization = await notarizationClient
        .createDynamic()
        // state.data: Current sensor readings
        // This field contains the actual measurement data that changes with each reading
        .withStringState(
            JSON.stringify(initialReading),
            initialMetadata,
        )
        // immutable_description: Device information that never changes
        // Contains static information about the weather station hardware
        .withImmutableDescription(
            "Weather Station Model: WS-2024-HH01 | Sensors: DHT22 (Temp/Humidity), BMP280 (Pressure) | Installation: 2024-12-15",
        )
        // updatable_metadata: Operational status information
        // Contains information about device health, connectivity, and maintenance
        .withUpdatableMetadata(
            "Battery: 95% | Signal: Excellent | Last Calibration: 2024-12-20 | Status: Operational",
        )
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Weather station notarization created!");
    console.log(`🔗 Notarization ID: ${weatherNotarization.output.id}`);

    // Display initial state
    displayWeatherData("Initial Reading", weatherNotarization.output);

    // Simulate weather updates over time
    console.log("\n🔄 Simulating weather updates over time...\n");

    const weatherUpdates: WeatherUpdate[] = [
        { temp: 17.5, humidity: 62.0, pressure: 1012.8, description: "Morning reading - temperature rising" },
        { temp: 19.1, humidity: 58.0, pressure: 1011.2, description: "Midday reading - clear skies" },
        { temp: 18.3, humidity: 61.0, pressure: 1010.5, description: "Afternoon reading - slight cloud cover" },
    ];

    for (let i = 0; i < weatherUpdates.length; i++) {
        const update = weatherUpdates[i];
        const timestamp = now + ((i + 1) * 3600); // Each reading 1 hour later

        console.log(`📊 Update #${i + 1}: ${update.description}`);

        const newReading: WeatherReading = {
            temperature_celsius: update.temp,
            humidity_percent: update.humidity,
            pressure_hpa: update.pressure,
            timestamp: timestamp,
        };

        const newMetadata = `Location: Hamburg, Germany | Coordinates: 53.5488°N, 9.9872°E | Recorded: ${formatTimestamp(timestamp)
            }`;

        // Update the weather reading
        await notarizationClient
            .updateState(
                State.fromString(JSON.stringify(newReading), newMetadata),
                weatherNotarization.output.id,
            )
            .buildAndExecute(notarizationClient);

        console.log("✅ Reading updated successfully");

        // Retrieve and display updated notarization
        const updatedNotarization = await notarizationClient.readOnly().getNotarizationById(
            weatherNotarization.output.id,
        );

        displayWeatherData(`Update #${i + 1}`, updatedNotarization);

        // Every second update, also update the device status
        if (i % 2 === 1) {
            const batteryLevel = 95 - (i * 5); // Battery draining over time
            const signalStrength = batteryLevel > 80 ? "Excellent" : batteryLevel > 50 ? "Good" : "Fair";
            const newDeviceStatus =
                `Battery: ${batteryLevel}% | Signal: ${signalStrength} | Last Calibration: 2024-12-20 | Status: Operational`;

            await notarizationClient
                .updateMetadata(newDeviceStatus, weatherNotarization.output.id)
                .buildAndExecute(notarizationClient);

            console.log(`🔋 Device status updated: ${newDeviceStatus}`);
        }

        console.log("───────────────────────────────────────────\n");
    }

    console.log("🎯 Example Complete!");
    console.log("\n💡 Key Takeaways:");
    console.log("• state.data: Contains the actual sensor readings (JSON format)");
    console.log("• state.metadata: Provides context like location and timestamp");
    console.log("• immutable_description: Static device information that never changes");
    console.log("• updatable_metadata: Dynamic operational info (battery, signal, maintenance)");
    console.log("\nThis separation allows efficient updates while maintaining data integrity!");
}

/** Helper function to display weather data in a structured format */
function displayWeatherData(title: string, notarization: OnChainNotarization): void {
    console.log(`📋 ${title}`);
    console.log("─────────────────");

    try {
        // Parse and display state.data (sensor readings)
        const readingData = JSON.parse(notarization.state.data.toString());
        console.log(`🌡️  Temperature: ${readingData.temperature_celsius}°C`);
        console.log(`💧 Humidity: ${readingData.humidity_percent}%`);
        console.log(`📊 Pressure: ${readingData.pressure_hpa} hPa`);

        // Display state.metadata (measurement context)
        if (notarization.state.metadata) {
            console.log(`📍 Context: ${notarization.state.metadata}`);
        }

        // Display immutable_description (device info)
        if (notarization.immutableMetadata.description) {
            console.log(`🔧 Device: ${notarization.immutableMetadata.description}`);
        }

        // Display updatable_metadata (device status)
        if (notarization.updatableMetadata) {
            console.log(`⚡ Status: ${notarization.updatableMetadata}`);
        }

        console.log(
            `🔢 Version: ${notarization.stateVersionCount} | 🕐 Updated: ${formatTimestamp(Math.floor(Number(notarization.lastStateChangeAt) / 1000))
            }`,
        );
    } catch (error) {
        console.error("Error displaying weather data:", error);
    }
}

/** Helper function to format Unix timestamp as readable date */
function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toISOString().replace("T", " ").replace(/\.\d{3}Z$/, " UTC");
}
