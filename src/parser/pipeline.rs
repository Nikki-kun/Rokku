use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::Result;

use crate::data::{is_timestamp_field, LogInfoEntry, LogInfoKind, ParamGroup, ParsedLog};

use super::ulog;
use flightlog_core::{Log, ReadOptions};

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

pub fn parse_log(filename: &str) -> Result<ParsedLog> {
    if is_ulog_file(filename) {
        return ulog::parse_ulog_columnar(filename);
    }
    log::debug!("Parsing ArduPilot log: {}", filename);
    let bytes = std::fs::read(filename)?;
    let log = Log::read(
        &bytes,
        &ReadOptions {
            keep_order: true,
            ..ReadOptions::default()
        },
    )?;
    let mut series_names: Vec<String> = Vec::new();
    let mut series_values: Vec<Vec<(u64, f64)>> = Vec::new();
    let mut series_idx_by_table: Vec<Vec<Option<usize>>> = Vec::new();
    let discovered_types: BTreeSet<String> = log.counts.keys().cloned().collect();
    let mut total_points = 0usize;

    for table in log.tables() {
        let mut field_indexes = Vec::with_capacity(table.columns.len());
        for column in &table.columns {
            if is_timestamp_field(&column.name) {
                field_indexes.push(None);
                continue;
            }
            let idx = series_names.len();
            series_names.push(format!("{}.{}", table.name, column.name));
            series_values.push(Vec::new());
            field_indexes.push(Some(idx));
        }
        series_idx_by_table.push(field_indexes);
    }

    let mut last_timestamp = 0u64;
    for &(t, row) in log.order.as_deref().unwrap_or_default() {
        let (table, row) = (&log.tables()[t as usize], row as usize);
        let current_ts = match table.column("TimeUS").and_then(|c| c.data.get_f64(row)) {
            Some(us) if us > 0.0 => {
                last_timestamp = us as u64 * 1000;
                last_timestamp
            }
            _ => last_timestamp,
        };
        for (column, idx) in table.columns.iter().zip(&series_idx_by_table[t as usize]) {
            if let (Some(idx), Some(v)) = (idx, column.data.get_f64(row)) {
                if v.is_finite() {
                    series_values[*idx].push((current_ts, v));
                    total_points += 1;
                }
            }
        }
    }

    if !log.integrity.is_clean() {
        log::warn!(
            "{}: skipped {} damaged bytes in {} regions",
            filename,
            log.integrity.skipped_bytes,
            log.integrity.skipped_region_count
        );
    }
    let parm_values: BTreeMap<String, String> = log
        .params
        .iter()
        .map(|(k, v)| (k.clone(), format!("{:.6}", v)))
        .collect();

    log::debug!(
        "Parsed {} data points into {} series",
        total_points,
        series_values.iter().filter(|v| !v.is_empty()).count()
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

    let mut series: HashMap<String, Vec<(u64, f64)>> = HashMap::with_capacity(series_names.len());
    for (name, values) in series_names.into_iter().zip(series_values.into_iter()) {
        if !values.is_empty() {
            series.insert(name, values);
        }
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

    fn fmt(type_id: u8, len: u8, name: &str, format: &str, labels: &str) -> Vec<u8> {
        let mut m = vec![0xA3, 0x95, 0x80, type_id, len];
        for (text, width) in [(name, 4), (format, 16), (labels, 64)] {
            let mut field = text.as_bytes().to_vec();
            field.resize(width, 0);
            m.extend(field);
        }
        m
    }

    fn gps(time_us: u64, lat: i32, alt_cm: i16) -> Vec<u8> {
        let mut m = vec![0xA3, 0x95, 200];
        m.extend(time_us.to_le_bytes());
        m.extend(lat.to_le_bytes());
        m.extend(alt_cm.to_le_bytes());
        m
    }

    #[test]
    fn test_dataflash_scaled_and_reads_past_damage() {
        let mut bytes = fmt(0x80, 89, "FMT", "BBnNZ", "Type,Length,Name,Format,Columns");
        bytes.extend(fmt(200, 17, "GPS", "QLc", "TimeUS,Lat,Alt"));
        bytes.extend(gps(1_000_000, 407_127_760, 1234));
        bytes.extend([0x00, 0xFF, 0x13]);
        bytes.extend(gps(2_000_000, 407_127_770, 1240));

        let path = std::env::temp_dir().join("rokku_dataflash_test.bin");
        std::fs::write(&path, &bytes).unwrap();
        let parsed = parse_log(path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);

        let lat = &parsed.series["GPS.Lat"];
        assert_eq!(lat.len(), 2);
        assert!((lat[0].1 - 40.712776).abs() < 1e-9);
        assert_eq!(lat[1].0, 2_000_000_000);
        assert!((parsed.series["GPS.Alt"][1].1 - 12.40).abs() < 1e-9);
    }
}
