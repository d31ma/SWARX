use crate::models::{NativeProcess, NativeProcessMetrics};

#[derive(Default)]
#[Service]
pub struct NativeCounterService;

impl NativeCounterService {
    const PERCENT: f64 = 100.0;
    const ZERO: f64 = 0.0;

    pub fn measure(
        &self,
        process: &NativeProcess,
        previous: Option<&NativeProcess>,
        network: (Option<(i64, i64)>, Option<(i64, i64)>),
        elapsed: Option<f64>,
    ) -> NativeProcessMetrics {
        let previous: Option<&NativeProcess> = self.baseline(process, previous);
        let network_previous: Option<(i64, i64)> = previous.and(network.1);
        let duration: Option<f64> = self.duration(elapsed);
        let previous_cpu: Option<f64> =
            previous.and_then(|value: &NativeProcess| -> Option<f64> { value.cpu_seconds });
        let cpu_valid: bool = self.finite_cpu(process.cpu_seconds, previous_cpu);
        let cpu: Option<f64> =
            NativeCounterUtils::delta(process.cpu_seconds, previous_cpu, duration, cpu_valid);
        let incoming: Option<i64> = NativeCounterUtils::delta(
            network.0.map(|value: (i64, i64)| -> i64 { value.0 }),
            network_previous.map(|value: (i64, i64)| -> i64 { value.0 }),
            duration,
            true,
        );
        let outgoing: Option<i64> = NativeCounterUtils::delta(
            network.0.map(|value: (i64, i64)| -> i64 { value.1 }),
            network_previous.map(|value: (i64, i64)| -> i64 { value.1 }),
            duration,
            true,
        );
        let disk_read: Option<i64> = NativeCounterUtils::delta(
            process.disk_read_bytes,
            previous.and_then(|value: &NativeProcess| -> Option<i64> { value.disk_read_bytes }),
            duration,
            true,
        );
        let disk_write: Option<i64> = NativeCounterUtils::delta(
            process.disk_written_bytes,
            previous.and_then(|value: &NativeProcess| -> Option<i64> { value.disk_written_bytes }),
            duration,
            true,
        );
        NativeProcessMetrics {
            pid: process.pid,
            parent: process.parent,
            started: process.started.clone(),
            name: process
                .executable
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_owned(),
            cpu_percent: self
                .rate(cpu, duration)
                .map(|value: f64| -> f64 { value * Self::PERCENT }),
            resident_bytes: process.resident_bytes,
            received_per_second: self
                .rate(incoming.map(|value: i64| -> f64 { value as f64 }), duration),
            sent_per_second: self
                .rate(outgoing.map(|value: i64| -> f64 { value as f64 }), duration),
            disk_read_per_second: self.rate(
                disk_read.map(|value: i64| -> f64 { value as f64 }),
                duration,
            ),
            disk_write_per_second: self.rate(
                disk_write.map(|value: i64| -> f64 { value as f64 }),
                duration,
            ),
            cpu_seconds_delta: cpu,
            received_bytes_delta: incoming,
            sent_bytes_delta: outgoing,
            disk_read_bytes_delta: disk_read,
            disk_written_bytes_delta: disk_write,
        }
    }

    fn baseline<'a>(
        &self,
        process: &NativeProcess,
        previous: Option<&'a NativeProcess>,
    ) -> Option<&'a NativeProcess> {
        previous.filter(|previous: &&'a NativeProcess| -> bool {
            [
                process.pid == previous.pid,
                process.started == previous.started,
                process.usage_start == previous.usage_start,
            ]
            .into_iter()
            .all(std::convert::identity)
        })
    }

    fn duration(&self, elapsed: Option<f64>) -> Option<f64> {
        elapsed.filter(|value: &f64| -> bool {
            [value.is_finite(), *value > Self::ZERO]
                .into_iter()
                .all(std::convert::identity)
        })
    }

    fn finite_cpu(&self, current: Option<f64>, previous: Option<f64>) -> bool {
        [current, previous]
            .into_iter()
            .flatten()
            .all(f64::is_finite)
    }

    fn rate(&self, delta: Option<f64>, duration: Option<f64>) -> Option<f64> {
        delta
            .zip(duration)
            .map(|(delta, duration): (f64, f64)| -> f64 { delta / duration })
    }
}
