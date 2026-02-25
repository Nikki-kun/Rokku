use anyhow::Result;
use serde_json::Map;

use super::reader::{ArduFrame, ArduReader};

/// Parsed log record with timestamp, message type and field values.
#[derive(Debug, Clone)]
pub struct LogRecord {
    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,
    /// Message type name (e.g. "GPS", "ATT", "IMU")
    pub message_type: String,
    /// Raw type ID from the log format
    pub type_id: u8,
    /// Field name -> value mapping
    pub fields: Map<String, serde_json::Value>,
}

/// Parse ArduPilot Dataflash log file into a vector of log records.
pub fn parse_log(filename: &str) -> Result<Vec<LogRecord>> {
    let mut reader = ArduReader::new(filename);
    let mut records = Vec::new();

    loop {
        match reader.read()? {
            ArduFrame::Eof => break,
            ArduFrame::ArduDefinition(_) => {}
            ArduFrame::ArduMessage(message) => {
                let definition = reader
                    .get_definition(&message.type_id)
                    .ok_or_else(|| anyhow::anyhow!("Unknown type_id {}", message.type_id))?;

                records.push(LogRecord {
                    timestamp_ns: message.current_ts,
                    message_type: definition.ardu_fmt.name.clone(),
                    type_id: message.type_id,
                    fields: message.json_obj,
                });
            }
        }
    }

    Ok(records)
}
