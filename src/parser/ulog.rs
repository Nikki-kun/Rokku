use std::fs::File;
use std::io::BufReader;

use anyhow::Result;
use serde_json::{Map, Value};
use ulog_rs::{ULogParser, ULogValue};

use super::LogRecord;

pub fn parse_ulog(filename: &str) -> Result<Vec<LogRecord>> {
    log::debug!("Parsing ULog file: {}", filename);
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let parser = ULogParser::parse_reader(reader)?;

    let mut records = Vec::new();
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
            let mut fields = Map::new();
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

                let is_ts_field = ts_field_idx.map_or(field_idx == 0, |idx| field_idx == idx);
                if is_ts_field {
                    if let Some(ns) = ulog_value_to_timestamp_ns(val) {
                        timestamp_ns = ns;
                    }
                }

                if let Some(json_val) = ulog_value_to_json(val) {
                    fields.insert(field.field_name.clone(), json_val);
                }
            }

            records.push(LogRecord {
                timestamp_ns,
                message_type: msg_name.clone(),
                type_id: sub.msg_id as u8,
                fields,
            });
        }
    }

    records.sort_by_key(|r| r.timestamp_ns);
    log::debug!("Parsed {} ULog records", records.len());
    Ok(records)
}

fn ulog_value_to_timestamp_ns(v: &ULogValue) -> Option<u64> {
    match v {
        ULogValue::UInt64(us) => Some(*us * 1000),
        ULogValue::Int64(is) if *is >= 0 => Some(*is as u64 * 1000),
        _ => None,
    }
}

fn ulog_value_to_json(v: &ULogValue) -> Option<Value> {
    match v {
        ULogValue::Int8(x) => Some(Value::Number((*x as i64).into())),
        ULogValue::UInt8(x) => Some(Value::Number((*x as i64).into())),
        ULogValue::Int16(x) => Some(Value::Number((*x as i64).into())),
        ULogValue::UInt16(x) => Some(Value::Number((*x as i64).into())),
        ULogValue::Int32(x) => Some(Value::Number((*x as i64).into())),
        ULogValue::UInt32(x) => Some(Value::Number((*x as i64).into())),
        ULogValue::Int64(x) => Some(Value::Number((*x).into())),
        ULogValue::UInt64(x) => Some(Value::Number((*x as i64).into())),
        ULogValue::Float(x) if x.is_finite() => {
            serde_json::Number::from_f64(*x as f64).map(Value::Number)
        }
        ULogValue::Double(x) if x.is_finite() => {
            serde_json::Number::from_f64(*x).map(Value::Number)
        }
        ULogValue::Bool(x) => Some(Value::Bool(*x)),
        ULogValue::Char(x) => Some(Value::String(x.to_string())),
        ULogValue::CharArray(s) => Some(Value::String(s.clone())),
        ULogValue::FloatArray(arr) if !arr.is_empty() && arr[0].is_finite() => {
            serde_json::Number::from_f64(arr[0] as f64).map(Value::Number)
        }
        ULogValue::DoubleArray(arr) if !arr.is_empty() && arr[0].is_finite() => {
            serde_json::Number::from_f64(arr[0]).map(Value::Number)
        }
        ULogValue::Int32Array(arr) if !arr.is_empty() => {
            Some(Value::Number((arr[0] as i64).into()))
        }
        ULogValue::UInt32Array(arr) if !arr.is_empty() => {
            Some(Value::Number((arr[0] as i64).into()))
        }
        _ => None,
    }
}
