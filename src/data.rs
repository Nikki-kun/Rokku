use std::collections::BTreeSet;

use crate::parser::LogRecord;

pub const MAX_CHART_POINTS: usize = 250;

pub fn downsample_series(data: &[(f64, f64)], max_points: usize) -> Vec<(f64, f64)> {
    if data.len() <= max_points || data.len() < 3 {
        return data.to_vec();
    }

    let n_buckets = max_points - 2;
    let mut result = Vec::with_capacity(max_points);
    result.push(data[0]);

    let bucket_size = (data.len() - 2) as f64 / n_buckets as f64;

    for i in 0..n_buckets {
        let bucket_start = 1 + (i as f64 * bucket_size) as usize;
        let bucket_end = 1 + ((i + 1) as f64 * bucket_size) as usize;
        let bucket_end = bucket_end.min(data.len() - 1);

        if bucket_start >= bucket_end {
            continue;
        }

        let next_bucket_start = bucket_end;
        let next_bucket_end = 1 + ((i + 2) as f64 * bucket_size) as usize;
        let next_bucket_end = next_bucket_end.min(data.len());
        let next_count = (next_bucket_end - next_bucket_start).max(1);
        let (next_sum_x, next_sum_y) = data[next_bucket_start..next_bucket_end]
            .iter()
            .fold((0.0, 0.0), |(sx, sy), (x, y)| (sx + x, sy + y));
        let next_avg = (
            next_sum_x / next_count as f64,
            next_sum_y / next_count as f64,
        );

        let prev = *result.last().unwrap();

        let mut max_area = 0.0f64;
        let mut best_idx = bucket_start;

        for j in bucket_start..bucket_end {
            let point = data[j];
            let area = ((point.0 - prev.0) * (next_avg.1 - prev.1)
                - (next_avg.0 - prev.0) * (point.1 - prev.1))
                .abs();
            if area > max_area {
                max_area = area;
                best_idx = j;
            }
        }

        result.push(data[best_idx]);
    }

    result.push(*data.last().unwrap());
    result
}

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
    params
        .into_iter()
        .filter(|p| {
            extract_param_series(records, p).map_or(false, |s| !s.is_empty())
        })
        .collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_record(ts_ns: u64, msg_type: &str, field: &str, value: f64) -> LogRecord {
        LogRecord {
            timestamp_ns: ts_ns,
            message_type: msg_type.to_string(),
            type_id: 0,
            fields: [(field.to_string(), json!(value))].into_iter().collect(),
        }
    }

    #[test]
    fn test_downsample_series_returns_all_when_under_threshold() {
        let data: Vec<(f64, f64)> = (0..100).map(|i| (i as f64, i as f64 * 2.0)).collect();
        let result = downsample_series(&data, 500);
        assert_eq!(result.len(), 100);
        assert_eq!(result, data);
    }

    #[test]
    fn test_downsample_series_reduces_points() {
        let data: Vec<(f64, f64)> = (0..5000).map(|i| (i as f64, i as f64)).collect();
        let result = downsample_series(&data, 250);
        assert!(result.len() <= 250);
        assert_eq!(result[0], (0.0, 0.0));
        assert_eq!(result.last().copied(), Some((4999.0, 4999.0)));
    }

    #[test]
    fn test_downsample_series_preserves_first_and_last() {
        let data = vec![(1.0, 10.0), (2.0, 20.0), (3.0, 30.0), (4.0, 40.0), (5.0, 50.0)];
        let result = downsample_series(&data, 3);
        assert_eq!(result[0], (1.0, 10.0));
        assert_eq!(result.last().copied(), Some((5.0, 50.0)));
    }

    #[test]
    fn test_extract_time_range() {
        let records = vec![
            make_record(1_000_000, "A", "x", 1.0),
            make_record(5_000_000, "A", "x", 2.0),
            make_record(3_000_000, "A", "x", 1.5),
        ];
        let (min, max) = extract_time_range(&records);
        assert!((min - 1.0).abs() < 1e-9);
        assert!((max - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_extract_time_range_empty() {
        let (min, max) = extract_time_range(&[]);
        assert_eq!(min, 0.0);
        assert_eq!(max, 1.0);
    }

    #[test]
    fn test_extract_parameters() {
        let records = vec![
            make_record(0, "GPS", "Lat", 55.0),
            make_record(0, "GPS", "Lng", 37.0),
            make_record(0, "ATT", "Roll", 0.1),
        ];
        let params = extract_parameters(&records);
        assert_eq!(params.len(), 3);
        assert!(params.contains(&"ATT.Roll".to_string()));
        assert!(params.contains(&"GPS.Lat".to_string()));
        assert!(params.contains(&"GPS.Lng".to_string()));
    }

    #[test]
    fn test_extract_param_series() {
        let records = vec![
            make_record(1_000_000, "A", "v", 10.0),
            make_record(2_000_000, "A", "v", 20.0),
            make_record(3_000_000, "B", "v", 99.0),
        ];
        let series = extract_param_series(&records, "A.v").unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series[0], (1.0, 10.0));
        assert_eq!(series[1], (2.0, 20.0));
    }

    #[test]
    fn test_value_at_time_interpolation() {
        let records = vec![
            make_record(1_000_000, "A", "v", 10.0),
            make_record(3_000_000, "A", "v", 30.0),
        ];
        assert_eq!(value_at_time(&records, "A.v", 0.5), Some(10.0));
        assert_eq!(value_at_time(&records, "A.v", 2.0), Some(20.0));
        assert_eq!(value_at_time(&records, "A.v", 4.0), Some(30.0));
    }
}
