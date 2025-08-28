// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # IoT Weather Station Example - Dynamic Notarization
//!
//! This example demonstrates how to use notarization fields for a real-world IoT weather station
//! that continuously reports temperature and humidity readings.
//!
//! ## Field Usage Strategy:
//!
//! - **state.data**: Current sensor readings (temperature, humidity)
//! - **state.metadata**: Measurement context (location, timestamp, sensor info)
//! - **immutable_description**: Weather station device information (model, sensors)
//! - **updatable_metadata**: Device status and operational info (battery, signal, maintenance)
//!
//! This showcases how dynamic notarizations can evolve over time while maintaining
//! a clear separation of concerns between different types of data.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use examples::get_funded_client;
use notarization::core::types::State;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🌡️ IoT Weather Station - Dynamic Notarization Example");
    println!("=====================================================\n");

    let notarization_client = get_funded_client().await?;

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    println!("📡 Creating weather station notarization...");

    // Create initial weather reading
    let initial_reading = json!({
        "temperature_celsius": 16.2,
        "humidity_percent": 65.0,
        "pressure_hpa": 1013.25,
        "timestamp": now
    });

    let initial_metadata = format!(
        "Location: Hamburg, Germany | Coordinates: 53.5488°N, 9.9872°E | Recorded: {}",
        format_timestamp(now)
    );

    // Create dynamic notarization for weather station
    let weather_notarization = notarization_client
        .create_dynamic_notarization()
        // state.data: Current sensor readings
        // This field contains the actual measurement data that changes with each reading
        .with_string_state(
            initial_reading.to_string(),
            Some(initial_metadata.clone())
        )
        // immutable_description: Device information that never changes
        // Contains static information about the weather station hardware
        .with_immutable_description(
            "Weather Station Model: WS-2024-HH01 | Sensors: DHT22 (Temp/Humidity), BMP280 (Pressure) | Installation: 2024-12-15".to_string()
        )
        // updatable_metadata: Operational status information
        // Contains information about device health, connectivity, and maintenance
        .with_updatable_metadata(
            "Battery: 95% | Signal: Excellent | Last Calibration: 2024-12-20 | Status: Operational".to_string()
        )
        .finish()
        .build_and_execute(&notarization_client)
        .await?;

    let notarization_id = weather_notarization.output.id.object_id();
    println!("✅ Weather station notarization created!");
    println!("🔗 Notarization ID: {}", notarization_id);

    // Display initial state
    display_weather_data("Initial Reading", &weather_notarization.output)?;

    // Simulate weather updates over time
    println!("\n🔄 Simulating weather updates over time...\n");

    let weather_updates = [
        (17.5, 62.0, 1012.8, "Morning reading - temperature rising"),
        (19.1, 58.0, 1011.2, "Midday reading - clear skies"),
        (18.3, 61.0, 1010.5, "Afternoon reading - slight cloud cover"),
    ];

    for (i, (temp, humidity, pressure, description)) in weather_updates.iter().enumerate() {
        let timestamp = now + ((i + 1) as u64 * 3600); // Each reading 1 hour later

        println!("📊 Update #{}: {}", i + 1, description);

        let new_reading = json!({
            "temperature_celsius": temp,
            "humidity_percent": humidity,
            "pressure_hpa": pressure,
            "timestamp": timestamp
        });

        let new_metadata = format!(
            "Location: Hamburg, Germany | Coordinates: 53.5488°N, 9.9872°E | Recorded: {}",
            format_timestamp(timestamp)
        );

        // Update the weather reading
        let _ = notarization_client
            .update_state(
                State::from_string(new_reading.to_string(), Some(new_metadata)),
                *notarization_id,
            )
            .build_and_execute(&notarization_client)
            .await?;

        println!("✅ Reading updated successfully");

        // Retrieve and display updated notarization
        let updated_notarization = notarization_client.get_notarization_by_id(*notarization_id).await?;

        display_weather_data(&format!("Update #{}", i + 1), &updated_notarization)?;

        // Every second update, also update the device status
        if i % 2 == 1 {
            let battery_level = 95 - (i * 5); // Battery draining over time
            let new_device_status = format!(
                "Battery: {}% | Signal: {} | Last Calibration: 2024-12-20 | Status: Operational",
                battery_level,
                if battery_level > 80 {
                    "Excellent"
                } else if battery_level > 50 {
                    "Good"
                } else {
                    "Fair"
                }
            );

            notarization_client
                .update_metadata(Some(new_device_status.clone()), *notarization_id)
                .build_and_execute(&notarization_client)
                .await?;

            println!("🔋 Device status updated: {}", new_device_status);
        }

        println!("───────────────────────────────────────────\n");
    }

    println!("🎯 Example Complete!");
    println!("\n💡 Key Takeaways:");
    println!("• state.data: Contains the actual sensor readings (JSON format)");
    println!("• state.metadata: Provides context like location and timestamp");
    println!("• immutable_description: Static device information that never changes");
    println!("• updatable_metadata: Dynamic operational info (battery, signal, maintenance)");
    println!("\nThis separation allows efficient updates while maintaining data integrity!");

    Ok(())
}

/// Helper function to display weather data in a structured format
fn display_weather_data(title: &str, notarization: &notarization::core::types::OnChainNotarization) -> Result<()> {
    println!("📋 {}", title);
    println!("─────────────────");

    // Parse and display state.data (sensor readings)
    if let Ok(reading_data) = serde_json::from_str::<serde_json::Value>(&notarization.state.data.clone().as_text()?) {
        println!("🌡️  Temperature: {}°C", reading_data["temperature_celsius"]);
        println!("💧 Humidity: {}%", reading_data["humidity_percent"]);
        println!("📊 Pressure: {} hPa", reading_data["pressure_hpa"]);
    }

    // Display state.metadata (measurement context)
    if let Some(metadata) = notarization.state.metadata() {
        println!("📍 Context: {}", metadata);
    }

    // Display immutable_description (device info)
    if let Some(description) = notarization.immutable_metadata.description.clone() {
        println!("🔧 Device: {}", description);
    }

    // Display updatable_metadata (device status)
    if let Some(device_status) = &notarization.updatable_metadata {
        println!("⚡ Status: {}", device_status);
    }

    println!(
        "🔢 Version: {} | 🕐 Updated: {}",
        notarization.state_version_count,
        format_timestamp(notarization.last_state_change_at / 1000)
    );

    Ok(())
}

/// Helper function to format Unix timestamp as readable date
fn format_timestamp(timestamp: u64) -> String {
    use chrono::{DateTime, Utc};
    DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Invalid timestamp".to_string())
}
