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
    const CPU_CEILING: f64 = 100.0;
    const BLUE_TONE: &'static str = "blue";
    const VIOLET_TONE: &'static str = "violet";
    const AMBER_TONE: &'static str = "amber";
    const CPU: (&'static str, &'static str, usize, f64, &'static str) =
        (Self::CPU_MODE, "CPU", 1, 1.0, Self::BLUE_TONE);
    const MEMORY: (&'static str, &'static str, usize, f64, &'static str) = (
        "resident",
        "Resident memory",
        2,
        Self::MEBIBYTE,
        Self::VIOLET_TONE,
    );
    const RECEIVED: (&'static str, &'static str, usize, f64, &'static str) =
        ("received", "Received", 3, Self::KIBIBYTE, Self::BLUE_TONE);
    const SENT: (&'static str, &'static str, usize, f64, &'static str) =
        ("sent", "Sent", 4, Self::KIBIBYTE, Self::AMBER_TONE);
    const DISK_READ: (&'static str, &'static str, usize, f64, &'static str) =
        ("disk-read", "Disk read", 5, Self::KIBIBYTE, Self::BLUE_TONE);
    const DISK_WRITE: (&'static str, &'static str, usize, f64, &'static str) = (
        "disk-write",
        "Disk write",
        6,
        Self::KIBIBYTE,
        Self::VIOLET_TONE,
    );

    pub fn build(history: &[Vec<Option<f64>>], now: f64) -> Vec<NativeChartView> {
        let rows: Vec<(f64, &[Option<f64>])> = Self::valid_rows(history);
        vec![
            Self::view(&rows, now, Self::cpu_chart(), &[Self::CPU]),
            Self::view(&rows, now, Self::memory_chart(), &[Self::MEMORY]),
            Self::view(
                &rows,
                now,
                Self::network_chart(),
                &[Self::RECEIVED, Self::SENT],
            ),
            Self::view(
                &rows,
                now,
                Self::disk_chart(),
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

    fn cpu_chart() -> NativeChartView {
        NativeChartView {
            mode: Self::CPU_MODE.to_owned(),
            unit: "% of one logical core".to_owned(),
            empty_text: Self::WAITING.to_owned(),
            filled: true,
            legend: false,
            zero_based: true,
            ceiling: Some(Self::CPU_CEILING),
            ..NativeChartView::default()
        }
    }

    fn memory_chart() -> NativeChartView {
        NativeChartView {
            mode: "memory".to_owned(),
            unit: "MiB RSS".to_owned(),
            empty_text: Self::WAITING.to_owned(),
            filled: true,
            legend: false,
            zero_based: false,
            ceiling: None,
            ..NativeChartView::default()
        }
    }

    fn network_chart() -> NativeChartView {
        NativeChartView {
            mode: "network".to_owned(),
            unit: "KiB / s (partial)".to_owned(),
            empty_text: Self::NETWORK_MISSING.to_owned(),
            filled: false,
            legend: true,
            zero_based: true,
            ceiling: None,
            ..NativeChartView::default()
        }
    }

    fn disk_chart() -> NativeChartView {
        NativeChartView {
            mode: "disk".to_owned(),
            unit: "KiB / s (storage)".to_owned(),
            empty_text: Self::WAITING.to_owned(),
            filled: true,
            legend: true,
            zero_based: true,
            ceiling: None,
            ..NativeChartView::default()
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
        chart: NativeChartView,
        channels: &[(&str, &str, usize, f64, &str)],
    ) -> NativeChartView {
        let series: Vec<NativeChartSeries> = Self::series(rows, channels);
        let measured_count: i64 = Self::total_count(&series);
        let window_end: f64 = Self::last_time(rows, now);
        NativeChartView {
            window_start: window_end - Self::WINDOW_SECONDS,
            window_end,
            sample_count: rows.len() as i64,
            measured_count,
            empty: measured_count == Self::NO_MEASUREMENTS,
            series,
            ..chart
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
        channels: &[(&str, &str, usize, f64, &str)],
    ) -> Vec<NativeChartSeries> {
        let mut series: Vec<NativeChartSeries> = Vec::new();
        for channel in channels {
            series.push(Self::channel(rows, *channel));
        }
        series
    }

    fn channel(
        rows: &[(f64, &[Option<f64>])],
        channel: (&str, &str, usize, f64, &str),
    ) -> NativeChartSeries {
        let points: Vec<NativeChartPoint> = Self::points(rows, channel.2, channel.3);
        let measured_count: i64 = Self::point_count(&points);
        NativeChartSeries {
            id: channel.0.to_owned(),
            label: channel.1.to_owned(),
            points,
            measured_count,
            tone: channel.4.to_owned(),
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
