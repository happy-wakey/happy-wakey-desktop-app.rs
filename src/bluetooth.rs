use serde_json::json;

pub const SERVICE_UUID: &str = "8e0e0001-7d5a-4c3f-9c31-94e9d447fc01";
pub const COMMAND_UUID: &str = "8e0e0002-7d5a-4c3f-9c31-94e9d447fc01";
pub const SCHEMA: &str = "happy-wakey.ble.preview-command.v1";
pub const ACTION: &str = "preview_alarm";
pub const DURATION_MS: u32 = 3000;
pub const MAX_COMMAND_BYTES: usize = 512;

pub fn encode_preview_alarm_command(operation_id: &str) -> Result<Vec<u8>, String> {
    if !is_uuid(operation_id) {
        return Err("Bluetooth operation identifier must be a UUID".into());
    }
    let payload = json!({
        "schema": SCHEMA,
        "operation_id": operation_id.to_ascii_lowercase(),
        "action": ACTION,
        "duration_ms": DURATION_MS,
    });
    let bytes = serde_json::to_vec(&payload)
        .map_err(|_| "Bluetooth command could not be encoded".to_string())?;
    if bytes.len() > MAX_COMMAND_BYTES {
        return Err("Bluetooth command exceeded its byte limit".into());
    }
    Ok(bytes)
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.as_bytes().get(8) == Some(&b'-')
        && value.as_bytes().get(13) == Some(&b'-')
        && value.as_bytes().get(18) == Some(&b'-')
        && value.as_bytes().get(23) == Some(&b'-')
        && value
            .chars()
            .enumerate()
            .all(|(index, ch)| matches!(index, 8 | 13 | 18 | 23) || ch.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_command_is_versioned_bounded_and_credential_free() {
        let bytes = encode_preview_alarm_command("018f5cc6-6d8b-7b2a-9f38-269e6a7b1f11").unwrap();
        assert!(bytes.len() <= MAX_COMMAND_BYTES);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["schema"], SCHEMA);
        assert_eq!(value["action"], ACTION);
        assert_eq!(value["duration_ms"], DURATION_MS);
        assert_eq!(
            value["operation_id"],
            "018f5cc6-6d8b-7b2a-9f38-269e6a7b1f11"
        );
        let encoded = value.to_string();
        assert!(!encoded.contains("token"));
        assert!(!encoded.contains("subject"));
        assert!(!encoded.contains("owner_id"));
        assert_eq!(SERVICE_UUID.len(), 36);
        assert_eq!(COMMAND_UUID.len(), 36);
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
    }
}
