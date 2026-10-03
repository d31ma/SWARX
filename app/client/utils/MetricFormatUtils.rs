pub struct MetricFormatUtils;

impl MetricFormatUtils {
    const UNAVAILABLE: &'static str = "Unavailable";
    const PENDING: &'static str = "Baseline pending";
    const UNITS: [&'static str; 4] = ["B", "KiB", "MiB", "GiB"];
    const UNIT_SIZE: f64 = 1024.0;
    const UNIT_POWER: f64 = 10.0;
    const MINUTE_SECONDS: f64 = 60.0;
    const ONE_DECIMAL: usize = 1;
    const TWO_DECIMALS: usize = 2;
    const QUARTER: f64 = 0.25;
    const ROUND_UP_FRACTIONS: [[f64; 2]; 3] =
        [[-1.0, -1.0], [Self::QUARTER, Self::QUARTER], [0.125, 0.625]];
    const ZERO: f64 = 0.0;

    pub fn bytes(value: Option<f64>) -> String {
        let supported: Option<f64> = Self::supported(value);
        supported
            .map(Self::scaled_bytes)
            .unwrap_or_else(Self::unavailable)
    }

    pub fn rate(value: Option<f64>) -> String {
        let supported: Option<f64> = Self::supported(value);
        supported
            .map(Self::scaled_rate)
            .unwrap_or_else(Self::unavailable)
    }

    pub fn cpu(value: Option<f64>) -> String {
        let supported: Option<f64> = Self::supported(value);
        supported
            .map(Self::cpu_number)
            .unwrap_or_else(Self::pending)
    }

    pub fn elapsed(seconds: f64) -> String {
        let supported: Option<f64> = Self::supported(Some(seconds));
        supported
            .map(Self::elapsed_time)
            .unwrap_or_else(Self::unavailable)
    }

    pub fn core_seconds(value: Option<f64>) -> String {
        let supported: Option<f64> = Self::supported(value);
        supported
            .map(Self::core_number)
            .unwrap_or_else(Self::unavailable)
    }

    fn supported(value: Option<f64>) -> Option<f64> {
        value.filter(Self::valid_value)
    }

    fn valid_value(value: &f64) -> bool {
        [value.is_finite(), *value >= Self::ZERO]
            .into_iter()
            .all(std::convert::identity)
    }

    fn unavailable() -> String {
        Self::UNAVAILABLE.to_owned()
    }

    fn pending() -> String {
        Self::PENDING.to_owned()
    }

    fn scaled_bytes(value: f64) -> String {
        let power: usize = (value.max(1.0).log2() / Self::UNIT_POWER).floor() as usize;
        let index: usize = power.min(Self::UNITS.len() - 1);
        let scaled: f64 = value / Self::UNIT_SIZE.powi(index as i32);
        let precision: usize = index.min(Self::ONE_DECIMAL);
        let rounded: f64 = [scaled.round(), scaled][precision];
        let number: String = Self::fixed_number(rounded, precision);
        format!("{number} {}", Self::UNITS[index])
    }

    fn scaled_rate(value: f64) -> String {
        let bytes: String = Self::scaled_bytes(value);
        format!("{bytes}/s")
    }

    fn cpu_number(value: f64) -> String {
        let number: String = Self::fixed_number(value, Self::ONE_DECIMAL);
        format!("{number}%")
    }

    fn fixed_number(value: f64, precision: usize) -> String {
        let number: String = format!("{value:.precision$}");
        let round_up: bool = Self::ROUND_UP_FRACTIONS[precision].contains(&value.fract());
        if round_up {
            let last: usize = number.len() - 1;
            let digit: char = char::from(number.as_bytes()[last] + 1);
            return format!("{}{digit}", &number[..last]);
        }
        number
    }

    fn core_number(value: f64) -> String {
        let number: String = Self::fixed_number(value, Self::TWO_DECIMALS);
        format!("{number} core-s")
    }

    fn elapsed_time(seconds: f64) -> String {
        if seconds < Self::MINUTE_SECONDS {
            return format!("{:.0}s", seconds.floor());
        }
        let minutes: f64 = (seconds / Self::MINUTE_SECONDS).floor();
        let remainder: f64 = (seconds % Self::MINUTE_SECONDS).floor();
        format!("{minutes:.0}m {remainder:.0}s")
    }
}
