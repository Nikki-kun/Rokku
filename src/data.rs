//! Data extraction utilities for log records

use std::collections::BTreeSet;

use crate::parser::LogRecord;

/// Returns (min_t, max_t) in ms (same units as extract_param_series)
pub fn extract_time_range(records: &[LogRecord]) -> (f64, f64) {
    if records.is_empty() {
        return (0.0, 1.0);
    }
    let (min_ns, max_ns) = records
        .iter()
        .map(|r| r.timestamp_ns as f64)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), t| {
            (a.min(t), b.max(t))
        });
    (min_ns / 1_000_000.0, max_ns / 1_000_000.0)
}

/// Interpolated value at given time t (ms) for a parameter
pub fn value_at_time(records: &[LogRecord], param: &str, t: f64) -> Option<f64> {
    let series = extract_param_series(records, param)?;
    if series.is_empty() {
        return None;
    }
    let first_ts = series[0].0;
    let last_ts = series.last()?.0;
    if t <= first_ts {
        return Some(series[0].1);
    }
    if t >= last_ts {
        return Some(series.last()?.1);
    }
    for i in 0..series.len() - 1 {
        let (t1, v1) = series[i];
        let (t2, v2) = series[i + 1];
        if t >= t1 && t <= t2 {
            let frac = if (t2 - t1).abs() < 1e-9 {
                1.0
            } else {
                (t - t1) / (t2 - t1)
            };
            return Some(v1 + (v2 - v1) * frac);
        }
    }
    None
}

pub fn extract_parameters(records: &[LogRecord]) -> Vec<String> {
    let mut params = BTreeSet::new();
    for r in records {
        for field_name in r.fields.keys() {
            params.insert(format!("{}.{}", r.message_type, field_name));
        }
    }
    params.into_iter().collect()
}

/// Extract series for param "MessageType.FieldName"
pub fn extract_param_series(records: &[LogRecord], param: &str) -> Option<Vec<(f64, f64)>> {
    let mut parts = param.splitn(2, '.');
    let msg_type = parts.next()?;
    let field = parts.next()?;
    let mut points = Vec::new();
    for r in records {
        if r.message_type == msg_type {
            if let Some(v) = r.fields.get(field) {
                let val = v.as_f64().or_else(|| v.as_i64().map(|x| x as f64))?;
                points.push((r.timestamp_ns as f64 / 1_000_000.0, val));
            }
        }
    }
    if points.is_empty() {
        None
    } else {
        Some(points)
    }
}
