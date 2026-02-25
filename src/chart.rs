use plotters::element::DashedPathElement;
use plotters::prelude::*;
use plotters::style::Color;
use plotters_backend::DrawingBackend;
use plotters_iced::Chart;

use crate::app::Message;
use crate::data::{downsample_series, MAX_CHART_POINTS};

pub const CHART_PLOT_LEFT: f32 = 65.0;
pub const CHART_PLOT_RIGHT: f32 = 50.0;

const FERRA_GRID: RGBColor = RGBColor(0x4f, 0x47, 0x4d);
const FERRA_LABEL: RGBColor = RGBColor(0xab, 0x8a, 0x79);
const CHART_COLORS: [RGBColor; 5] = [
    RGBColor(0x5d, 0xc1, 0xc5),
    RGBColor(0xf6, 0xb6, 0xc9),
    RGBColor(0xb1, 0xb6, 0x95),
    RGBColor(0xff, 0xa0, 0x7a),
    RGBColor(0xd7, 0xbd, 0xe2),
];

pub fn format_axis_value(v: f64) -> String {
    let s = format!("{:.4}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

const CURSOR_LINE_COLOR: RGBColor = RGBColor(0xe8, 0xdc, 0xa8);

pub struct LogChart {
    data: Vec<(f64, f64)>,
    color_idx: usize,
    cursor_time: Option<f64>,
}

impl LogChart {
    pub fn with_cursor(
        _title: String,
        data: Vec<(f64, f64)>,
        color_idx: usize,
        cursor_time: Option<f64>,
    ) -> Self {
        let data = downsample_series(&data, MAX_CHART_POINTS);
        Self {
            data,
            color_idx: color_idx % CHART_COLORS.len(),
            cursor_time,
        }
    }
}

impl Chart<Message> for LogChart {
    type State = ();

    fn build_chart<DB: DrawingBackend>(
        &self,
        _state: &Self::State,
        mut chart: plotters_iced::ChartBuilder<DB>,
    ) {
        if self.data.is_empty() {
            return;
        }

        let (min_ts, max_ts): (f64, f64) = self
            .data
            .iter()
            .map(|(t, _)| *t)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), t| {
                (a.min(t), b.max(t))
            });

        let (min_val, max_val): (f64, f64) = self
            .data
            .iter()
            .map(|(_, v)| *v)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
                (a.min(v), b.max(v))
            });

        let val_margin = (max_val - min_val).abs().max(1.0) * 0.08;
        let val_range = (min_val - val_margin)..(max_val + val_margin);

        let mut chart = chart
            .x_label_area_size(35)
            .y_label_area_size(50)
            .margin(15)
            .build_cartesian_2d(min_ts..max_ts, val_range)
            .expect("build chart");

        let label_style = ("sans-serif", 11).into_font().color(&FERRA_LABEL);
        chart
            .configure_mesh()
            .light_line_style(FERRA_GRID.mix(0.15))
            .bold_line_style(FERRA_GRID.mix(0.35))
            .axis_style(ShapeStyle::from(&FERRA_LABEL.mix(0.8)).stroke_width(1))
            .label_style(label_style)
            .x_label_formatter(&|v| format!("{:.1}s", v / 1000.0))
            .y_label_formatter(&|v| format_axis_value(*v))
            .x_labels(8)
            .y_labels(6)
            .draw()
            .expect("draw mesh");

        let line_color = CHART_COLORS[self.color_idx];
        let fill_color = line_color.mix(0.12);

        chart
            .draw_series(
                AreaSeries::new(
                    self.data.iter().map(|(x, y)| (*x, *y)),
                    min_val - val_margin,
                    fill_color,
                )
                .border_style(ShapeStyle::from(&line_color).stroke_width(2)),
            )
            .expect("draw area");

        chart
            .draw_series(LineSeries::new(
                self.data.iter().copied(),
                ShapeStyle::from(&line_color).stroke_width(2),
            ))
            .expect("draw line");

        if let Some(ct) = self.cursor_time {
            if ct >= min_ts && ct <= max_ts {
                let y_min = min_val - val_margin;
                let y_max = max_val + val_margin;
                chart
                    .draw_series(std::iter::once(DashedPathElement::new(
                        vec![(ct, y_min), (ct, y_max)],
                        4,
                        4,
                        ShapeStyle::from(&CURSOR_LINE_COLOR).stroke_width(1),
                    )))
                    .expect("draw cursor line");
            }
        }
    }
}

/// Combined chart showing multiple series normalized to 0..1
pub struct CombinedLogChart {
    series: Vec<(String, Vec<(f64, f64)>)>,
    cursor_time: Option<f64>,
}

impl CombinedLogChart {
    pub fn new(
        series: Vec<(String, Vec<(f64, f64)>)>,
        cursor_time: Option<f64>,
    ) -> Self {
        let series: Vec<(String, Vec<(f64, f64)>)> = series
            .into_iter()
            .map(|(name, data)| {
                let data = downsample_series(&data, MAX_CHART_POINTS);
                (name, data)
            })
            .collect();
        Self { series, cursor_time }
    }
}

impl Chart<Message> for CombinedLogChart {
    type State = ();

    fn build_chart<DB: DrawingBackend>(
        &self,
        _state: &Self::State,
        mut chart: plotters_iced::ChartBuilder<DB>,
    ) {
        if self.series.is_empty() {
            return;
        }

        let (min_ts, max_ts): (f64, f64) = self
            .series
            .iter()
            .flat_map(|(_, d)| d.iter().map(|(t, _)| *t))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), t| (a.min(t), b.max(t)));

        let mut chart = chart
            .x_label_area_size(35)
            .y_label_area_size(50)
            .margin(15)
            .build_cartesian_2d(min_ts..max_ts, 0.0..1.0)
            .expect("build chart");

        let label_style = ("sans-serif", 11).into_font().color(&FERRA_LABEL);
        chart
            .configure_mesh()
            .light_line_style(FERRA_GRID.mix(0.15))
            .bold_line_style(FERRA_GRID.mix(0.35))
            .axis_style(ShapeStyle::from(&FERRA_LABEL.mix(0.8)).stroke_width(1))
            .label_style(label_style)
            .x_label_formatter(&|v| format!("{:.1}s", v / 1000.0))
            .y_label_formatter(&|v| format!("{:.2}", v))
            .x_labels(8)
            .y_labels(6)
            .draw()
            .expect("draw mesh");

        for (idx, (_, data)) in self.series.iter().enumerate() {
            if data.is_empty() {
                continue;
            }
            let (min_val, max_val): (f64, f64) = data
                .iter()
                .map(|(_, v)| *v)
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(v), b.max(v)));
            let range = (max_val - min_val).abs().max(1e-10);
            let normalized: Vec<(f64, f64)> = data
                .iter()
                .map(|(t, v)| (*t, (*v - min_val) / range))
                .collect();

            let line_color = CHART_COLORS[idx % CHART_COLORS.len()];
            chart
                .draw_series(LineSeries::new(
                    normalized.into_iter(),
                    ShapeStyle::from(&line_color).stroke_width(2),
                ))
                .expect("draw line");
        }

        if let Some(ct) = self.cursor_time {
            if ct >= min_ts && ct <= max_ts {
                chart
                    .draw_series(std::iter::once(DashedPathElement::new(
                        vec![(ct, 0.0), (ct, 1.0)],
                        4,
                        4,
                        ShapeStyle::from(&CURSOR_LINE_COLOR).stroke_width(1),
                    )))
                    .expect("draw cursor line");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_axis_value() {
        assert_eq!(format_axis_value(1.5), "1.5");
        assert_eq!(format_axis_value(1.0), "1");
        assert_eq!(format_axis_value(0.1234), "0.1234");
        assert_eq!(format_axis_value(100.0), "100");
    }
}
