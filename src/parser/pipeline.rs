use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::Result;

use crate::data::{is_timestamp_field, LogInfoEntry, LogInfoKind, ParamGroup, ParsedLog};

use super::reader::{ArduFrame, ArduReader, LogValue};
use super::ulog;

fn is_ulog_file(filename: &str) -> bool {
    std::path::Path::new(filename)
        .extension()
        .map(|e| e.eq_ignore_ascii_case("ulg"))
        .unwrap_or(false)
}

fn finish_parsed_log(
    mut series: HashMap<String, Vec<(u64, f64)>>,
    mut info_entries: Vec<LogInfoEntry>,
) -> ParsedLog {
    let mut min_ns = u64::MAX;
    let mut max_ns = 0u64;

    for data in series.values_mut() {
        data.sort_by_key(|(ns, _)| *ns);
        for &(ns, _) in data.iter() {
            min_ns = min_ns.min(ns);
            max_ns = max_ns.max(ns);
        }
    }

    let time_range = if min_ns <= max_ns {
        (min_ns as f64 / 1_000_000.0, max_ns as f64 / 1_000_000.0)
    } else {
        (0.0, 1.0)
    };

    let parameters: Vec<String> = series
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, _)| k.clone())
        .collect();

    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in &parameters {
        if let Some((name, field)) = p.split_once('.') {
            groups
                .entry(name.to_string())
                .or_default()
                .push(field.to_string());
        }
    }
    let parameter_groups: Vec<ParamGroup> = groups
        .into_iter()
        .map(|(name, mut fields)| {
            fields.sort();
            fields.dedup();
            ParamGroup { name, fields }
        })
        .collect();

    info_entries.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.cmp(&b.name)));

    ParsedLog {
        series,
        time_range,
        parameters,
        parameter_groups,
        info_entries,
    }
}

fn log_value_to_text(value: &LogValue) -> String {
    match value {
        LogValue::Int(v) => v.to_string(),
        LogValue::UInt(v) => v.to_string(),
        LogValue::Float(v) => format!("{:.6}", v),
        LogValue::Double(v) => format!("{:.6}", v),
        LogValue::Str(v) => v.clone(),
    }
}

pub fn parse_log(filename: &str) -> Result<ParsedLog> {
    if is_ulog_file(filename) {
        return ulog::parse_ulog_columnar(filename);
    }
    log::debug!("Parsing ArduPilot log: {}", filename);
    let mut reader = ArduReader::new(filename);
    let mut series: HashMap<String, Vec<(u64, f64)>> = HashMap::new();
    let mut discovered_types: BTreeSet<String> = BTreeSet::new();
    let mut parm_values: BTreeMap<String, String> = BTreeMap::new();
    let mut total_points = 0usize;

    loop {
        match reader.read()? {
            ArduFrame::Eof => break,
            ArduFrame::ArduDefinition(definition) => {
                discovered_types.insert(definition.name.to_string());
            }
            ArduFrame::ArduMessage(message) => {
                let definition = reader
                    .get_definition(&message.type_id)
                    .ok_or_else(|| anyhow::anyhow!("Unknown type_id {}", message.type_id))?;
                let msg_type = definition.name.as_ref();
                let fields = message.fields;

                if msg_type == "PARM" {
                    let mut parm_name: Option<String> = None;
                    let mut parm_value: Option<String> = None;

                    for (label, val) in &fields {
                        if label == "Name" {
                            if let LogValue::Str(v) = val {
                                if !v.is_empty() {
                                    parm_name = Some(v.clone());
                                }
                            }
                        } else if label == "Value" {
                            parm_value = Some(log_value_to_text(val));
                        }
                    }

                    if let (Some(name), Some(value)) = (parm_name, parm_value) {
                        parm_values.insert(name, value);
                    }
                }

                for (label, val) in fields {
                    if is_timestamp_field(&label) {
                        continue;
                    }
                    if let Some(v) = val.to_f64() {
                        let key = format!("{}.{}", msg_type, label);
                        series.entry(key).or_default().push((message.current_ts, v));
                        total_points += 1;
                    }
                }
            }
        }
    }

    log::debug!(
        "Parsed {} data points into {} series",
        total_points,
        series.len()
    );

    let mut info_entries = Vec::new();
    info_entries.push(LogInfoEntry {
        kind: LogInfoKind::Metadata,
        name: "log.format".to_string(),
        value: "ArduPilot Dataflash".to_string(),
    });
    info_entries.push(LogInfoEntry {
        kind: LogInfoKind::Metadata,
        name: "messages.defined_types".to_string(),
        value: discovered_types.len().to_string(),
    });
    for (name, value) in parm_values {
        info_entries.push(LogInfoEntry {
            kind: LogInfoKind::Parameter,
            name,
            value,
        });
    }

    Ok(finish_parsed_log(series, info_entries))
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
