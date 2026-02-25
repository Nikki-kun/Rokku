use std::collections::HashMap;

pub const MAX_CHART_POINTS: usize = 250;

const TIMESTAMP_FIELDS: &[&str] = &["TimeUS", "timestamp"];

#[derive(Clone, Debug, Default)]
pub struct ParsedLog {
    pub(crate) series: HashMap<String, Vec<(u64, f64)>>,
    pub(crate) time_range: (f64, f64),
    pub(crate) parameters: Vec<String>,
    pub(crate) parameter_groups: Vec<ParamGroup>,
}

impl ParsedLog {
    pub fn time_range(&self) -> (f64, f64) {
        if self.series.is_empty() {
            return (0.0, 1.0);
        }
        self.time_range
    }

    pub fn parameters(&self) -> &[String] {
        &self.parameters
    }

    pub fn parameter_groups(&self) -> &[ParamGroup] {
        &self.parameter_groups
    }

    pub fn get_series(&self, param: &str) -> Option<Vec<(f64, f64)>> {
        let data = self.series.get(param)?;
        if data.is_empty() {
            return None;
        }
        Some(
            data.iter()
                .map(|(ns, v)| (*ns as f64 / 1_000_000.0, *v))
                .collect(),
        )
    }

    pub fn value_at_time(&self, param: &str, t: f64) -> Option<f64> {
        let data = self.series.get(param)?;
        if data.is_empty() {
            return None;
        }
        let t_ns = (t * 1_000_000.0) as u64;
        let idx = data.partition_point(|(ns, _)| *ns < t_ns);
        if idx == 0 {
            return Some(data[0].1);
        }
        if idx >= data.len() {
            return Some(data[data.len() - 1].1);
        }
        let (t1, v1) = data[idx - 1];
        let (t2, v2) = data[idx];
        let t1_f = t1 as f64 / 1_000_000.0;
        let t2_f = t2 as f64 / 1_000_000.0;
        let frac = if (t2_f - t1_f).abs() < 1e-9 {
            1.0
        } else {
            (t - t1_f) / (t2_f - t1_f)
        };
        Some(v1 + (v2 - v1) * frac)
    }

    pub fn validate_param_series(&self, param: &str) -> Option<Vec<(f64, f64)>> {
        let series = self.get_series(param)?;
        if let Some(&(first_ts, _)) = series.first() {
            log::info!(
                "Param {}: first TimeUS/timestamp = {:.2} ms ({:.3} s)",
                param,
                first_ts,
                first_ts / 1000.0
            );
            log::info!("Param {}: {} points", param, series.len());
        }
        Some(series)
    }
}

pub fn is_timestamp_field(field_name: &str) -> bool {
    TIMESTAMP_FIELDS.contains(&field_name)
}

pub fn filter_series_to_range(
    data: &[(f64, f64)],
    range_start: f64,
    range_end: f64,
) -> Vec<(f64, f64)> {
    if data.is_empty() {
        return vec![];
    }
    let start_idx = data.partition_point(|(t, _)| *t < range_start);
    let end_idx = data.partition_point(|(t, _)| *t <= range_end);
    let start_idx = start_idx.saturating_sub(1);
    let end_idx = end_idx.min(data.len().saturating_sub(1));
    if start_idx > end_idx {
        return vec![];
    }
    data[start_idx..=end_idx].to_vec()
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

#[derive(Debug, Clone)]
pub struct ParamGroup {
    pub name: String,
    pub fields: Vec<String>,
}

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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use super::*;

    fn make_parsed_log(
        series: impl IntoIterator<Item = (&'static str, Vec<(u64, f64)>)>,
    ) -> ParsedLog {
        let series: HashMap<String, Vec<(u64, f64)>> = series
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let mut min_ns = u64::MAX;
        let mut max_ns = 0u64;
        for v in series.values() {
            for &(ns, _) in v {
                min_ns = min_ns.min(ns);
                max_ns = max_ns.max(ns);
            }
        }
        let time_range = if min_ns <= max_ns {
            (min_ns as f64 / 1_000_000.0, max_ns as f64 / 1_000_000.0)
        } else {
            (0.0, 1.0)
        };
        let parameters: Vec<String> = series.keys().cloned().collect();
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
        let data = vec![
            (1.0, 10.0),
            (2.0, 20.0),
            (3.0, 30.0),
            (4.0, 40.0),
            (5.0, 50.0),
        ];
        let result = downsample_series(&data, 3);
        assert_eq!(result[0], (1.0, 10.0));
        assert_eq!(result.last().copied(), Some((5.0, 50.0)));
    }

    #[test]
    fn test_time_range() {
        let log = make_parsed_log([(
            "A.x",
            vec![(1_000_000, 1.0), (5_000_000, 2.0), (3_000_000, 1.5)],
        )]);
        let (min, max) = log.time_range();
        assert!((min - 1.0).abs() < 1e-9);
        assert!((max - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_time_range_empty() {
        let log = ParsedLog::default();
        let (min, max) = log.time_range();
        assert_eq!(min, 0.0);
        assert_eq!(max, 1.0);
    }

    #[test]
    fn test_parameters() {
        let log = make_parsed_log([
            ("GPS.Lat", vec![(0, 55.0)]),
            ("GPS.Lng", vec![(0, 37.0)]),
            ("ATT.Roll", vec![(0, 0.1)]),
        ]);
        let params = log.parameters();
        assert_eq!(params.len(), 3);
        assert!(params.contains(&"ATT.Roll".to_string()));
        assert!(params.contains(&"GPS.Lat".to_string()));
        assert!(params.contains(&"GPS.Lng".to_string()));
    }

    #[test]
    fn test_parameters_excludes_timestamp_fields() {
        let log = make_parsed_log([
            ("AETR.SS", vec![(1_000_000, 5.0)]),
            ("GPS.Lat", vec![(2_000_000, 55.0)]),
        ]);
        let params = log.parameters();
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
    fn test_parameter_groups() {
        let log = make_parsed_log([
            ("AETR.SS", vec![(1_000_000, 5.0)]),
            ("AETR.Rudd", vec![(1_000_000, 10.0)]),
            ("GPS.Lat", vec![(2_000_000, 55.0)]),
            ("GPS.Lng", vec![(2_000_000, 37.0)]),
        ]);
        let groups = log.parameter_groups();
        assert_eq!(groups.len(), 2);
        let aetr = groups.iter().find(|g| g.name == "AETR").unwrap();
        assert_eq!(aetr.fields.len(), 2);
        assert!(aetr.fields.contains(&"Rudd".to_string()));
        assert!(aetr.fields.contains(&"SS".to_string()));
    }

    #[test]
    fn test_get_series() {
        let log = make_parsed_log([
            ("A.v", vec![(1_000_000, 10.0), (2_000_000, 20.0)]),
            ("B.v", vec![(3_000_000, 99.0)]),
        ]);
        let series = log.get_series("A.v").unwrap();
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
    fn test_get_series_non_zero_start() {
        let log = make_parsed_log([(
            "A.v",
            vec![
                (12_000_000_000, 1.0),
                (13_000_000_000, 2.0),
                (14_000_000_000, 3.0),
            ],
        )]);
        let series = log.get_series("A.v").unwrap();
        assert_eq!(series.len(), 3);
        assert_eq!(series[0].0, 12_000.0);
        assert_eq!(series[1].0, 13_000.0);
        assert_eq!(series[2].0, 14_000.0);
    }

    #[test]
    fn test_value_at_time_interpolation() {
        let log = make_parsed_log([("A.v", vec![(1_000_000, 10.0), (3_000_000, 30.0)])]);
        assert_eq!(log.value_at_time("A.v", 0.5), Some(10.0));
        assert_eq!(log.value_at_time("A.v", 2.0), Some(20.0));
        assert_eq!(log.value_at_time("A.v", 4.0), Some(30.0));
    }

    #[test]
    fn test_validate_param_series_match() {
        let _ = env_logger::try_init();
        let log = make_parsed_log([("A.v", vec![(1_000_000, 10.0), (2_000_000, 20.0)])]);
        let result = log.validate_param_series("A.v");
        assert_eq!(result.as_ref().map(|s| s.len()), Some(2));
        assert_eq!(result.unwrap()[0], (1.0, 10.0));
    }

    #[test]
    fn test_validate_param_series_mismatch() {
        let _ = env_logger::try_init();
        let log = make_parsed_log([
            ("A.v", vec![(1_000_000, 10.0), (3_000_000, 30.0)]),
            ("A.x", vec![(2_000_000, 99.0)]),
        ]);
        let result = log.validate_param_series("A.v");
        assert_eq!(result.as_ref().map(|s| s.len()), Some(2));
    }
}
