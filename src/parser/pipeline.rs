use anyhow::Result;
use serde_json::Map;

use super::reader::{ArduFrame, ArduReader};

#[derive(Debug, Clone)]
pub struct LogRecord {
    pub timestamp_ns: u64,
    pub message_type: String,
    pub type_id: u8,
    pub fields: Map<String, serde_json::Value>,
}

fn estimate_capacity(filename: &str) -> usize {
    std::fs::metadata(filename)
        .ok()
        .map(|m| ((m.len() / 50).min(500_000) as usize).max(1000))
        .unwrap_or(10_000)
}

pub fn parse_log(filename: &str) -> Result<Vec<LogRecord>> {
    log::debug!("Parsing log file: {}", filename);
    let capacity = estimate_capacity(filename);
    let mut reader = ArduReader::new(filename);
    let mut records = Vec::with_capacity(capacity);

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

    log::debug!("Parsed {} records", records.len());
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_nonexistent() {
        let result = parse_log("/nonexistent/path/log.bin");
        assert!(result.is_err());
    }
}
