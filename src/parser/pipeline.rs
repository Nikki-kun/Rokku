use std::collections::{BTreeMap, HashMap};

use anyhow::Result;

use crate::data::{is_timestamp_field, ParamGroup, ParsedLog};

use super::reader::{ArduFrame, ArduReader};
use super::ulog;

fn is_ulog_file(filename: &str) -> bool {
    std::path::Path::new(filename)
        .extension()
        .map(|e| e.eq_ignore_ascii_case("ulg"))
        .unwrap_or(false)
}

fn finish_parsed_log(mut series: HashMap<String, Vec<(u64, f64)>>) -> ParsedLog {
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

    ParsedLog {
        series,
        time_range,
        parameters,
        parameter_groups,
    }
}

pub fn parse_log(filename: &str) -> Result<ParsedLog> {
    if is_ulog_file(filename) {
        return ulog::parse_ulog_columnar(filename);
    }
    log::debug!("Parsing ArduPilot log: {}", filename);
    let mut reader = ArduReader::new(filename);
    let mut series: HashMap<String, Vec<(u64, f64)>> = HashMap::new();
    let mut total_points = 0usize;

    loop {
        match reader.read()? {
            ArduFrame::Eof => break,
            ArduFrame::ArduDefinition(_) => {}
            ArduFrame::ArduMessage(message) => {
                let definition = reader
                    .get_definition(&message.type_id)
                    .ok_or_else(|| anyhow::anyhow!("Unknown type_id {}", message.type_id))?;
                let msg_type = definition.name.as_ref();

                for (label, val) in message.fields {
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
    Ok(finish_parsed_log(series))
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
