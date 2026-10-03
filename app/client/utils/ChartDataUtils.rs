use crate::models::{NativeChartPoint, NativeChartSeries, NativeChartView, NativeRuntimeView};

pub struct ChartDataUtils;

impl ChartDataUtils {
    const WINDOW_SECONDS: f64 = 120.0;
    const MINIMUM_VALUE: f64 = 0.0;
    const KIBIBYTE: f64 = 1024.0;
    const MEBIBYTE: f64 = 1_048_576.0;
    const NO_MEASUREMENTS: i64 = 0;
    const WAITING: &'static str = "Waiting for measured samples.";
    const NETWORK_MISSING: &'static str = "No visible socket-rate samples for this group.";
    const CPU_MODE: &'static str = "cpu";
    const CPU: (&'static str, &'static str, usize, f64) = (Self::CPU_MODE, "CPU", 1, 1.0);
    const MEMORY: (&'static str, &'static str, usize, f64) =
        ("resident", "Resident memory", 2, Self::MEBIBYTE);
    const RECEIVED: (&'static str, &'static str, usize, f64) =
        ("received", "Received", 3, Self::KIBIBYTE);
    const SENT: (&'static str, &'static str, usize, f64) = ("sent", "Sent", 4, Self::KIBIBYTE);
    const DISK_READ: (&'static str, &'static str, usize, f64) =
        ("disk-read", "Disk read", 5, Self::KIBIBYTE);
    const DISK_WRITE: (&'static str, &'static str, usize, f64) =
        ("disk-write", "Disk write", 6, Self::KIBIBYTE);

    pub fn build(history: &[Vec<Option<f64>>], now: f64) -> Vec<NativeChartView> {
        let rows: Vec<(f64, &[Option<f64>])> = Self::valid_rows(history);
        vec![
            Self::view(
                &rows,
                now,
                (Self::CPU_MODE, "% of one logical core", Self::WAITING),
                &[Self::CPU],
            ),
            Self::view(
                &rows,
                now,
                ("memory", "MiB RSS", Self::WAITING),
                &[Self::MEMORY],
            ),
            Self::view(
                &rows,
                now,
                ("network", "KiB / s (partial)", Self::NETWORK_MISSING),
                &[Self::RECEIVED, Self::SENT],
            ),
            Self::view(
                &rows,
                now,
                ("disk", "KiB / s (storage)", Self::WAITING),
                &[Self::DISK_READ, Self::DISK_WRITE],
            ),
        ]
    }

    pub fn present(views: &mut [NativeRuntimeView], now: f64) -> () {
        Self::present_views(views, now);
    }

    fn present_views(views: &mut [NativeRuntimeView], now: f64) -> () {
        for view in views {
            view.charts = Self::build(&view.group.history, now);
        }
    }

    fn valid_rows(history: &[Vec<Option<f64>>]) -> Vec<(f64, &[Option<f64>])> {
        history.iter().filter_map(Self::valid_row).collect()
    }

    fn valid_row(row: &Vec<Option<f64>>) -> Option<(f64, &[Option<f64>])> {
        let time: f64 = row.first().copied().flatten().filter(Self::finite)?;
        Some((time, row.as_slice()))
    }

    fn finite(value: &f64) -> bool {
        value.is_finite()
    }

    fn view(
        rows: &[(f64, &[Option<f64>])],
        now: f64,
        labels: (&str, &str, &str),
        channels: &[(&str, &str, usize, f64)],
    ) -> NativeChartView {
        let series: Vec<NativeChartSeries> = Self::series(rows, channels);
        let measured_count: i64 = Self::total_count(&series);
        let window_end: f64 = Self::last_time(rows, now);
        NativeChartView {
            mode: labels.0.to_owned(),
            unit: labels.1.to_owned(),
            window_start: window_end - Self::WINDOW_SECONDS,
            window_end,
            sample_count: rows.len() as i64,
            measured_count,
            empty: measured_count == Self::NO_MEASUREMENTS,
            empty_text: labels.2.to_owned(),
            series,
        }
    }

    fn last_time(rows: &[(f64, &[Option<f64>])], now: f64) -> f64 {
        rows.last().map(Self::row_time).unwrap_or(now)
    }

    fn row_time(row: &(f64, &[Option<f64>])) -> f64 {
        row.0
    }

    fn series(
        rows: &[(f64, &[Option<f64>])],
        channels: &[(&str, &str, usize, f64)],
    ) -> Vec<NativeChartSeries> {
        let mut series: Vec<NativeChartSeries> = Vec::new();
        for channel in channels {
            series.push(Self::channel(rows, *channel));
        }
        series
    }

    fn channel(
        rows: &[(f64, &[Option<f64>])],
        channel: (&str, &str, usize, f64),
    ) -> NativeChartSeries {
        let points: Vec<NativeChartPoint> = Self::points(rows, channel.2, channel.3);
        let measured_count: i64 = Self::point_count(&points);
        NativeChartSeries {
            id: channel.0.to_owned(),
            label: channel.1.to_owned(),
            points,
            measured_count,
        }
    }

    fn points(rows: &[(f64, &[Option<f64>])], index: usize, divisor: f64) -> Vec<NativeChartPoint> {
        let mut points: Vec<NativeChartPoint> = Vec::new();
        for row in rows {
            points.push(NativeChartPoint {
                x: row.0,
                y: Self::value(row.1, index, divisor),
            });
        }
        points
    }

    fn value(row: &[Option<f64>], index: usize, divisor: f64) -> Option<f64> {
        let value: Option<f64> = row
            .get(index)
            .copied()
            .flatten()
            .filter(Self::finite)
            .filter(Self::nonnegative);
        value.map(|value: f64| -> f64 { value / divisor })
    }

    fn nonnegative(value: &f64) -> bool {
        *value >= Self::MINIMUM_VALUE
    }

    fn point_count(points: &[NativeChartPoint]) -> i64 {
        points.iter().filter(Self::measured).count() as i64
    }

    fn measured(point: &&NativeChartPoint) -> bool {
        point.y.is_some()
    }

    fn total_count(series: &[NativeChartSeries]) -> i64 {
        series.iter().map(Self::series_count).sum()
    }

    fn series_count(series: &NativeChartSeries) -> i64 {
        series.measured_count
    }
}
