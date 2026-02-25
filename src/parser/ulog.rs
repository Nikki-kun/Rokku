use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use ulog_rs::{ULogParser, ULogValue};

use crate::data::{ParamGroup, ParsedLog};

pub fn parse_ulog_columnar(filename: &str) -> Result<ParsedLog> {
    log::debug!("Parsing ULog file: {}", filename);
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let parser = ULogParser::parse_reader(reader)?;

    let mut series: HashMap<String, Vec<(u64, f64)>> = HashMap::new();
    let mut total_points = 0usize;

    for sub in parser.subscriptions().values() {
        let msg_name = &sub.message_name;
        let data = &sub.data;
        let fields_info = &sub.format.fields;

        if data.is_empty() {
            continue;
        }

        let ts_field_idx = fields_info.iter().position(|f| f.field_name == "timestamp");

        let n_samples = data[0].len();
        for sample_idx in 0..n_samples {
            let mut timestamp_ns: u64 = 0;
            for (field_idx, field) in fields_info.iter().enumerate() {
                if field.field_name.starts_with("_padding") {
                    continue;
                }
                let Some(field_data) = data.get(field_idx) else {
                    continue;
                };
                let Some(val) = field_data.get(sample_idx) else {
                    continue;
                };
                let is_ts = field.field_name == "timestamp"
                    || ts_field_idx.map_or(field_idx == 0, |i| field_idx == i);
                if is_ts {
                    if let Some(ns) = ulog_value_to_timestamp_ns(val) {
                        timestamp_ns = ns;
                    }
                    continue;
                }
                if let Some(v) = ulog_value_to_f64(val) {
                    let key = format!("{}.{}", msg_name, field.field_name);
                    series.entry(key).or_default().push((timestamp_ns, v));
                    total_points += 1;
                }
            }
        }
    }

    log::debug!(
        "Parsed {} data points into {} series",
        total_points,
        series.len()
    );

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

    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
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

    Ok(ParsedLog {
        series,
        time_range,
        parameters,
        parameter_groups,
    })
}

fn ulog_value_to_timestamp_ns(v: &ULogValue) -> Option<u64> {
    match v {
        ULogValue::UInt64(us) => Some(*us * 1000),
        ULogValue::Int64(is) if *is >= 0 => Some(*is as u64 * 1000),
        _ => None,
    }
}

fn ulog_value_to_f64(v: &ULogValue) -> Option<f64> {
    match v {
        ULogValue::Int8(x) => Some(*x as f64),
        ULogValue::UInt8(x) => Some(*x as f64),
        ULogValue::Int16(x) => Some(*x as f64),
        ULogValue::UInt16(x) => Some(*x as f64),
        ULogValue::Int32(x) => Some(*x as f64),
        ULogValue::UInt32(x) => Some(*x as f64),
        ULogValue::Int64(x) => Some(*x as f64),
        ULogValue::UInt64(x) => Some(*x as f64),
        ULogValue::Float(x) if x.is_finite() => Some(*x as f64),
        ULogValue::Double(x) if x.is_finite() => Some(*x),
        ULogValue::Bool(x) => Some(if *x { 1.0 } else { 0.0 }),
        ULogValue::FloatArray(arr) if !arr.is_empty() && arr[0].is_finite() => Some(arr[0] as f64),
        ULogValue::DoubleArray(arr) if !arr.is_empty() && arr[0].is_finite() => Some(arr[0]),
        ULogValue::Int32Array(arr) if !arr.is_empty() => Some(arr[0] as f64),
        ULogValue::UInt32Array(arr) if !arr.is_empty() => Some(arr[0] as f64),
        _ => None,
    }
}
