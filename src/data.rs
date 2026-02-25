use std::collections::BTreeSet;

use crate::parser::LogRecord;

pub const MAX_CHART_POINTS: usize = 250;

/// Поля с временными метками — не показываем в выборе данных для графиков.
const TIMESTAMP_FIELDS: &[&str] = &["TimeUS", "timestamp"];

pub fn is_timestamp_field(field_name: &str) -> bool {
    TIMESTAMP_FIELDS.contains(&field_name)
}

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
            if !is_timestamp_field(field_name) {
                params.insert(format!("{}.{}", r.message_type, field_name));
            }
        }
    }
    params
        .into_iter()
        .filter(|p| {
            extract_param_series(records, p).map_or(false, |s| !s.is_empty())
        })
        .collect()
}

/// Группирует параметры по префиксу (MessageType). AETR.SS, AETR.Rudd → группа AETR.
#[derive(Debug, Clone)]
pub struct ParamGroup {
    pub name: String,
    pub fields: Vec<String>,
}

pub fn extract_parameter_groups(records: &[LogRecord]) -> Vec<ParamGroup> {
    let params = extract_parameters(records);
    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for p in params {
        if let Some((name, field)) = p.split_once('.') {
            groups
                .entry(name.to_string())
                .or_default()
                .push(field.to_string());
        }
    }
    groups
        .into_iter()
        .map(|(name, mut fields)| {
            fields.sort();
            fields.dedup();
            ParamGroup { name, fields }
        })
        .collect()
}

/// Извлекает серию (время в мс, значение). Время — абсолютное (TimeUS/timestamp),
/// может начинаться не с нуля (напр. с 12 с).
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
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Some(points)
    }
}

/// Добавляет точку (0, 0) в начало серии, если данные начинаются не с нуля.
/// Используется только для отображения на графике.
pub fn pad_series_from_zero(series: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if let Some(&(first_ts, _)) = series.first() {
        if first_ts > 0.0 {
            let mut padded = vec![(0.0, 0.0)];
            padded.extend(series);
            padded
        } else {
            series.to_vec()
        }
    } else {
        series.to_vec()
    }
}

/// Проверяет, что количество временных меток совпадает с количеством значений,
/// и логирует результат. Вызывается при добавлении графика.
pub fn validate_param_series_and_log(records: &[LogRecord], param: &str) -> Option<Vec<(f64, f64)>> {
    let mut parts = param.splitn(2, '.');
    let msg_type = parts.next()?;
    let _ = parts.next()?;

    let n_records_with_ts: usize = records
        .iter()
        .filter(|r| r.message_type == msg_type)
        .count();

    let series = extract_param_series(records, param)?;
    let n_points = series.len();

    if let Some((first_ts_ms, _)) = series.first() {
        log::info!(
            "Param {}: first TimeUS/timestamp = {:.2} ms ({:.3} s)",
            param,
            first_ts_ms,
            first_ts_ms / 1000.0
        );
    }

    if n_points != n_records_with_ts {
        log::error!(
            "Param {}: mismatch - {} timestamps (TimeUS/timestamp) vs {} values",
            param,
            n_records_with_ts,
            n_points
        );
    } else {
        log::info!(
            "Param {}: {} points, timestamps and values match",
            param,
            n_points
        );
    }

    Some(series)
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

    fn make_record_multi(
        ts_ns: u64,
        msg_type: &str,
        fields: impl IntoIterator<Item = (String, serde_json::Value)>,
    ) -> LogRecord {
        LogRecord {
            timestamp_ns: ts_ns,
            message_type: msg_type.to_string(),
            type_id: 0,
            fields: fields.into_iter().collect(),
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
    fn test_extract_parameters_excludes_timestamp_fields() {
        let records = vec![
            make_record_multi(
                1_000_000,
                "AETR",
                [
                    ("TimeUS".to_string(), json!(1_000_000)),
                    ("SS".to_string(), json!(5.0)),
                ],
            ),
            make_record_multi(
                2_000_000,
                "GPS",
                [
                    ("timestamp".to_string(), json!(2_000_000)),
                    ("Lat".to_string(), json!(55.0)),
                ],
            ),
        ];
        let params = extract_parameters(&records);
        assert!(!params.iter().any(|p| p.ends_with(".TimeUS")));
        assert!(!params.iter().any(|p| p.ends_with(".timestamp")));
        assert!(params.contains(&"AETR.SS".to_string()));
        assert!(params.contains(&"GPS.Lat".to_string()));
    }

    #[test]
    fn test_is_timestamp_field() {
        assert!(is_timestamp_field("TimeUS"));
        assert!(is_timestamp_field("timestamp"));
        assert!(!is_timestamp_field("SS"));
        assert!(!is_timestamp_field("Lat"));
    }

    #[test]
    fn test_extract_parameter_groups() {
        let records = vec![
            make_record_multi(
                1_000_000,
                "AETR",
                [
                    ("TimeUS".to_string(), json!(1_000_000)),
                    ("SS".to_string(), json!(5.0)),
                    ("Rudd".to_string(), json!(10.0)),
                ],
            ),
            make_record_multi(
                2_000_000,
                "GPS",
                [
                    ("timestamp".to_string(), json!(2_000_000)),
                    ("Lat".to_string(), json!(55.0)),
                    ("Lng".to_string(), json!(37.0)),
                ],
            ),
        ];
        let groups = extract_parameter_groups(&records);
        assert_eq!(groups.len(), 2);
        let aetr = groups.iter().find(|g| g.name == "AETR").unwrap();
        assert_eq!(aetr.fields.len(), 2);
        assert!(aetr.fields.contains(&"Rudd".to_string()));
        assert!(aetr.fields.contains(&"SS".to_string()));
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
    fn test_pad_series_from_zero() {
        let series = vec![(12_000.0, 1.0), (13_000.0, 2.0)];
        let padded = pad_series_from_zero(&series);
        assert_eq!(padded.len(), 3);
        assert_eq!(padded[0], (0.0, 0.0));
        assert_eq!(padded[1], (12_000.0, 1.0));
        assert_eq!(padded[2], (13_000.0, 2.0));

        let series_from_zero = vec![(0.0, 5.0), (1.0, 6.0)];
        let not_padded = pad_series_from_zero(&series_from_zero);
        assert_eq!(not_padded.len(), 2);
    }

    #[test]
    fn test_extract_param_series_non_zero_start() {
        // TimeUS/timestamp могут начинаться не с нуля (напр. 12 с)
        let records = vec![
            make_record(12_000_000_000, "A", "v", 1.0),  // 12 s
            make_record(13_000_000_000, "A", "v", 2.0),  // 13 s
            make_record(14_000_000_000, "A", "v", 3.0),  // 14 s
        ];
        let series = extract_param_series(&records, "A.v").unwrap();
        assert_eq!(series.len(), 3);
        assert_eq!(series[0].0, 12_000.0); // 12 s в мс
        assert_eq!(series[1].0, 13_000.0);
        assert_eq!(series[2].0, 14_000.0);
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

    #[test]
    fn test_validate_param_series_match() {
        let _ = env_logger::try_init();
        let records = vec![
            make_record(1_000_000, "A", "v", 10.0),
            make_record(2_000_000, "A", "v", 20.0),
        ];
        let result = validate_param_series_and_log(&records, "A.v");
        assert_eq!(result.as_ref().map(|s| s.len()), Some(2));
        assert_eq!(result.unwrap()[0], (1.0, 10.0));
    }

    #[test]
    fn test_validate_param_series_mismatch() {
        let _ = env_logger::try_init();
        // 3 records of type A, but only 2 have field "v"
        let records = vec![
            make_record(1_000_000, "A", "v", 10.0),
            make_record(2_000_000, "A", "x", 99.0), // no "v"
            make_record(3_000_000, "A", "v", 30.0),
        ];
        let result = validate_param_series_and_log(&records, "A.v");
        assert_eq!(result.as_ref().map(|s| s.len()), Some(2));
        // 3 timestamps (records) vs 2 values (points with field v)
    }
}
