//! Native Bluetooth Low Energy support for Happy Wakey alarm peripherals.
//!
//! Only devices advertising the product service UUID are discovered, and the
//! app writes only the versioned command characteristic. Bluetooth transport
//! never carries Shared Auth credentials or server-side identity attributes.

use std::{collections::BTreeMap, time::Duration};

use btleplug::{
    api::{Central, CharPropFlags, Manager as _, Peripheral as _, ScanFilter, WriteType},
    platform::{Manager, Peripheral},
};
use serde::Serialize;
use uuid::Uuid;

pub const SERVICE_UUID_STR: &str = "8e0e0001-7d5a-4c3f-9c31-94e9d447fc01";
pub const COMMAND_UUID_STR: &str = "8e0e0002-7d5a-4c3f-9c31-94e9d447fc01";
pub const SCHEMA: &str = "happy-wakey.ble.preview-command.v1";
pub const ACTION: &str = "preview_alarm";
pub const DURATION_MS: u32 = 3000;
pub const MAX_COMMAND_BYTES: usize = 512;

const SERVICE_UUID: Uuid = Uuid::from_u128(0x8e0e_0001_7d5a_4c3f_9c31_94e9_d447_fc01);
const COMMAND_UUID: Uuid = Uuid::from_u128(0x8e0e_0002_7d5a_4c3f_9c31_94e9_d447_fc01);
const SCAN_TIME: Duration = Duration::from_secs(4);
const CONNECT_TIME: Duration = Duration::from_secs(8);

#[derive(Clone, Debug, Serialize)]
pub struct DeviceSummary {
    pub id: String,
    pub name: String,
    pub rssi: Option<i16>,
    pub connected: bool,
}

#[derive(Serialize)]
struct PreviewCommand<'a> {
    schema: &'static str,
    operation_id: &'a str,
    action: &'static str,
    duration_ms: u16,
}

pub fn scan() -> Result<Vec<DeviceSummary>, String> {
    runtime()?.block_on(scan_async())
}

pub fn connect(device_id: &str) -> Result<DeviceSummary, String> {
    validate_device_id(device_id)?;
    runtime()?.block_on(async {
        let peripheral = find_peripheral(device_id).await?;
        ensure_connected(&peripheral).await?;
        ensure_happy_wakey_service(&peripheral)?;
        summarize(&peripheral).await
    })
}

pub fn disconnect(device_id: &str) -> Result<(), String> {
    validate_device_id(device_id)?;
    runtime()?.block_on(async {
        let peripheral = find_peripheral(device_id).await?;
        if peripheral
            .is_connected()
            .await
            .map_err(|error| format!("read Bluetooth connection state: {error}"))?
        {
            peripheral
                .disconnect()
                .await
                .map_err(|error| format!("disconnect Bluetooth device: {error}"))?;
        }
        Ok(())
    })
}

pub fn send_preview_alarm(device_id: &str) -> Result<(), String> {
    validate_device_id(device_id)?;
    let operation_id = Uuid::new_v4().to_string();
    let payload = preview_payload(&operation_id)?;
    runtime()?.block_on(async {
        let peripheral = find_peripheral(device_id).await?;
        ensure_connected(&peripheral).await?;
        ensure_happy_wakey_service(&peripheral)?;
        let characteristic = peripheral
            .characteristics()
            .into_iter()
            .find(|characteristic| {
                characteristic.uuid == COMMAND_UUID
                    && characteristic.properties.contains(CharPropFlags::WRITE)
            })
            .ok_or_else(|| {
                "Happy Wakey command characteristic is unavailable or not writable".to_string()
            })?;
        peripheral
            .write(&characteristic, &payload, WriteType::WithResponse)
            .await
            .map_err(|error| format!("write Bluetooth preview alarm: {error}"))
    })
}

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("start Bluetooth runtime: {error}"))
}

async fn scan_async() -> Result<Vec<DeviceSummary>, String> {
    let manager = Manager::new()
        .await
        .map_err(|error| format!("initialize Bluetooth manager: {error}"))?;
    let adapters = manager
        .adapters()
        .await
        .map_err(|error| format!("enumerate Bluetooth adapters: {error}"))?;
    if adapters.is_empty() {
        return Err("No Bluetooth adapter is available".into());
    }

    let mut devices = BTreeMap::new();
    for adapter in adapters {
        adapter
            .start_scan(ScanFilter {
                services: vec![SERVICE_UUID],
            })
            .await
            .map_err(|error| format!("start Happy Wakey Bluetooth scan: {error}"))?;
        tokio::time::sleep(SCAN_TIME).await;
        let discovered = adapter
            .peripherals()
            .await
            .map_err(|error| format!("read discovered Bluetooth devices: {error}"))?;
        let _ = adapter.stop_scan().await;
        for peripheral in discovered {
            let Some(properties) = peripheral
                .properties()
                .await
                .map_err(|error| format!("read Bluetooth advertisement: {error}"))?
            else {
                continue;
            };
            if !properties.services.contains(&SERVICE_UUID) {
                continue;
            }
            let summary = DeviceSummary {
                id: peripheral.id().to_string(),
                name: properties
                    .local_name
                    .or(properties.advertisement_name)
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| "Happy Wakey device".into()),
                rssi: properties.rssi,
                connected: peripheral.is_connected().await.unwrap_or(false),
            };
            devices.insert(summary.id.clone(), summary);
        }
    }
    Ok(devices.into_values().collect())
}

async fn find_peripheral(device_id: &str) -> Result<Peripheral, String> {
    let manager = Manager::new()
        .await
        .map_err(|error| format!("initialize Bluetooth manager: {error}"))?;
    for adapter in manager
        .adapters()
        .await
        .map_err(|error| format!("enumerate Bluetooth adapters: {error}"))?
    {
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|error| format!("read known Bluetooth devices: {error}"))?;
        if let Some(peripheral) = peripherals
            .into_iter()
            .find(|peripheral| peripheral.id().to_string() == device_id)
        {
            return Ok(peripheral);
        }
    }
    Err("Bluetooth device is no longer discoverable; scan again".into())
}

async fn ensure_connected(peripheral: &Peripheral) -> Result<(), String> {
    if !peripheral
        .is_connected()
        .await
        .map_err(|error| format!("read Bluetooth connection state: {error}"))?
    {
        peripheral
            .connect_with_timeout(CONNECT_TIME)
            .await
            .map_err(|error| format!("connect Bluetooth device: {error}"))?;
    }
    peripheral
        .discover_services()
        .await
        .map_err(|error| format!("discover Bluetooth services: {error}"))
}

fn ensure_happy_wakey_service(peripheral: &Peripheral) -> Result<(), String> {
    if peripheral
        .services()
        .iter()
        .any(|service| service.uuid == SERVICE_UUID)
    {
        Ok(())
    } else {
        Err("Connected peripheral does not implement the Happy Wakey BLE service".into())
    }
}

async fn summarize(peripheral: &Peripheral) -> Result<DeviceSummary, String> {
    let properties = peripheral
        .properties()
        .await
        .map_err(|error| format!("read Bluetooth device properties: {error}"))?;
    let (name, rssi) = properties
        .map(|properties| {
            (
                properties
                    .local_name
                    .or(properties.advertisement_name)
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| "Happy Wakey device".into()),
                properties.rssi,
            )
        })
        .unwrap_or_else(|| ("Happy Wakey device".into(), None));
    Ok(DeviceSummary {
        id: peripheral.id().to_string(),
        name,
        rssi,
        connected: peripheral.is_connected().await.unwrap_or(false),
    })
}

fn validate_device_id(device_id: &str) -> Result<(), String> {
    if device_id.is_empty()
        || device_id.len() > 256
        || device_id.chars().any(char::is_control)
        || device_id.trim() != device_id
    {
        return Err("Bluetooth device identifier is malformed".into());
    }
    Ok(())
}

pub fn encode_preview_alarm_command(operation_id: &str) -> Result<Vec<u8>, String> {
    preview_payload(operation_id)
}

fn preview_payload(operation_id: &str) -> Result<Vec<u8>, String> {
    let operation_id = Uuid::parse_str(operation_id)
        .map_err(|_| "Bluetooth operation identifier must be a UUID".to_string())?;
    let operation_id = operation_id.to_string();
    let payload = serde_json::to_vec(&PreviewCommand {
        schema: SCHEMA,
        operation_id: &operation_id,
        action: ACTION,
        duration_ms: DURATION_MS as u16,
    })
    .map_err(|error| format!("serialize Bluetooth command: {error}"))?;
    if payload.len() > MAX_COMMAND_BYTES {
        return Err("Bluetooth command exceeded its byte limit".into());
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_command_is_versioned_bounded_and_credential_free() {
        let payload = preview_payload("018f5cc6-6d8b-7b2a-9f38-269e6a7b1f11").unwrap();
        assert!(payload.len() <= MAX_COMMAND_BYTES);
        let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(value["schema"], "happy-wakey.ble.preview-command.v1");
        assert_eq!(value["action"], "preview_alarm");
        assert!(value.get("token").is_none());
        assert!(value.get("subject").is_none());
        assert!(value.get("owner_id").is_none());
    }

    #[test]
    fn device_identifier_is_strict_and_bounded() {
        assert!(validate_device_id("adapter/device-1").is_ok());
        assert!(validate_device_id("").is_err());
        assert!(validate_device_id(" device ").is_err());
        assert!(validate_device_id("device\n2").is_err());
        assert!(validate_device_id(&"x".repeat(257)).is_err());
    }

    #[test]
    fn preview_command_rejects_malformed_operation_identifiers() {
        assert!(encode_preview_alarm_command("not-an-operation-id").is_err());
        assert!(encode_preview_alarm_command("").is_err());
        assert!(encode_preview_alarm_command("018f5cc6-6d8b-7b2a-9f38-269e6a7b1f1").is_err());
        let bytes = encode_preview_alarm_command("018F5CC6-6D8B-7B2A-9F38-269E6A7B1F11").unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            value["operation_id"],
            "018f5cc6-6d8b-7b2a-9f38-269e6a7b1f11"
        );
        assert_eq!(SERVICE_UUID_STR.len(), 36);
        assert_eq!(COMMAND_UUID_STR.len(), 36);
    }
}
