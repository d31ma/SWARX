use observatory_measurement_tests::models::*;
use observatory_measurement_tests::*;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

fn process(pid: i32, parent: i32, name: &str, cpu: f64) -> NativeProcess {
    NativeProcess {
        pid,
        parent,
        started: "1700000000:123456".to_owned(),
        executable: name.to_owned(),
        user: 501,
        cpu_seconds: Some(cpu),
        resident_bytes: Some(1_000),
        disk_read_bytes: Some(2_000),
        disk_written_bytes: Some(3_000),
        usage_start: Some(40),
    }
}

fn group(processes: &[NativeProcess]) -> Vec<NativeGroup> {
    NativeProcessGroupService::default().group(processes)
}

#[test]
fn first_sample_rates_are_missing() -> () {
    let current = process(10, 1, "/usr/local/bin/codex", 5.0);
    let measured =
        NativeCounterService::default().measure(&current, None, (Some((10, 20)), None), None);
    assert_eq!(measured.cpu_percent, None);
    assert_eq!(measured.disk_read_per_second, None);
    assert_eq!(measured.received_per_second, None);
    assert_eq!(measured.resident_bytes, Some(1_000));
}

#[test]
fn delta_rates_have_correct_units() -> () {
    let previous = process(10, 1, "/usr/local/bin/codex", 5.0);
    let mut current = previous.clone();
    current.cpu_seconds = Some(6.0);
    current.disk_read_bytes = Some(2_100);
    current.disk_written_bytes = Some(3_400);
    let measured = NativeCounterService::default().measure(
        &current,
        Some(&previous),
        (Some((500, 800)), Some((100, 200))),
        Some(2.0),
    );
    assert_eq!(measured.cpu_percent, Some(50.0));
    assert_eq!(measured.disk_read_per_second, Some(50.0));
    assert_eq!(measured.disk_write_per_second, Some(200.0));
    assert_eq!(measured.received_per_second, Some(200.0));
    assert_eq!(measured.sent_per_second, Some(300.0));
}

#[test]
fn pid_reuse_invalidates_all_deltas() -> () {
    let previous = process(10, 1, "/usr/local/bin/codex", 5.0);
    let mut current = previous.clone();
    current.started = "1700000001:0".to_owned();
    current.cpu_seconds = Some(10.0);
    let measured = NativeCounterService::default().measure(
        &current,
        Some(&previous),
        (Some((500, 800)), Some((100, 200))),
        Some(2.0),
    );
    assert_eq!(measured.cpu_percent, None);
    assert_eq!(measured.disk_read_per_second, None);
    assert_eq!(measured.received_per_second, None);
}

#[test]
fn native_start_epoch_change_invalidates_all_deltas() -> () {
    let previous = process(10, 1, "/usr/local/bin/codex", 5.0);
    let mut current = previous.clone();
    current.usage_start = Some(41);
    let measured = NativeCounterService::default().measure(
        &current,
        Some(&previous),
        (Some((500, 800)), Some((100, 200))),
        Some(2.0),
    );
    assert_eq!(measured.cpu_percent, None);
    assert_eq!(measured.received_per_second, None);
}

#[test]
fn reset_counters_are_missing_and_idle_is_zero() -> () {
    let previous = process(10, 1, "/usr/local/bin/codex", 5.0);
    let mut current = previous.clone();
    current.cpu_seconds = Some(4.0);
    current.disk_read_bytes = Some(1);
    let measured = NativeCounterService::default().measure(
        &current,
        Some(&previous),
        (Some((1, 800)), Some((100, 200))),
        Some(2.0),
    );
    assert_eq!(measured.cpu_percent, None);
    assert_eq!(measured.disk_read_per_second, None);
    assert_eq!(measured.received_per_second, None);
    let idle = NativeCounterService::default().measure(
        &previous,
        Some(&previous),
        (None, None),
        Some(2.0),
    );
    assert_eq!(idle.cpu_percent, Some(0.0));
}

#[test]
fn invalid_elapsed_interval_is_missing() -> () {
    let current = process(10, 1, "/usr/local/bin/codex", 5.0);
    let measured = NativeCounterService::default().measure(
        &current,
        Some(&current),
        (Some((1, 1)), Some((1, 1))),
        Some(0.0),
    );
    assert_eq!(measured.cpu_percent, None);
    assert_eq!(measured.received_per_second, None);
}

#[test]
fn incidental_provider_text_does_not_classify_unknown_processes() -> () {
    let current = process(10, 1, "/tmp/claude-notes/python", 0.0);
    assert!(NativeProviderService::default()
        .classify(&current)
        .is_none());
    assert!(group(&[current]).is_empty());
}

#[test]
fn known_desktop_executable_is_shared_context() -> () {
    let current = process(
        10,
        1,
        "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT",
        0.0,
    );
    let provider = NativeProviderService::default().classify(&current).unwrap();
    assert_eq!(provider.family, "Codex");
    assert!(provider.shared);
}

#[test]
fn nested_cli_and_shared_runtime_are_disjoint() -> () {
    let processes = [
        process(
            10,
            1,
            "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT",
            0.0,
        ),
        process(20, 10, "/usr/local/bin/codex", 0.0),
        process(30, 20, "/bin/zsh", 0.0),
        process(40, 10, "/bin/sh", 0.0),
    ];
    let groups = group(&processes);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].members, vec![20, 30]);
    assert_eq!(groups[1].members, vec![10, 40]);
    assert_eq!(groups[0].status, "process-present");
    assert_eq!(groups[0].task_status, "unavailable");
    assert!(!groups[0].instrumented);
}

#[test]
fn parent_cycles_and_self_parent_do_not_duplicate_members() -> () {
    let groups = group(&[
        process(10, 20, "/usr/local/bin/codex", 0.0),
        process(20, 10, "/bin/sh", 0.0),
        process(30, 30, "/usr/local/bin/claude", 0.0),
    ]);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].members, vec![10, 20]);
    assert_eq!(groups[1].members, vec![30]);
}

#[test]
fn missing_parent_does_not_link_unrelated_process() -> () {
    let groups = group(&[
        process(10, 99, "/usr/local/bin/codex", 0.0),
        process(20, 98, "/bin/sh", 0.0),
    ]);
    assert_eq!(groups[0].members, vec![10]);
}

#[test]
fn csv_handles_quoted_names_columns_and_repeated_headers() -> () {
    let text = ",time,bytes_out,bytes_in\n\"codex,app.123\",1,20,10\n,time,bytes_in,bytes_out\nclaude.456,2,30,40\n";
    let counters = NativeNetworkCsvService::default().parse(text).unwrap();
    assert_eq!(counters, BTreeMap::from([(456, (30, 40))]));
}

#[test]
fn csv_rejects_missing_headers_and_bad_counters() -> () {
    assert!(NativeNetworkCsvService::default()
        .parse("permission denied")
        .is_err());
    let counters = NativeNetworkCsvService::default()
        .parse(",bytes_in,bytes_out\ncodex.10,-1,2\nclaude.notpid,1,2\ncodex.11,3,4\n")
        .unwrap();
    assert_eq!(counters, BTreeMap::from([(11, (3, 4))]));
}

#[test]
fn network_gap_clears_baseline_and_recovers_without_spike() -> () {
    let mut state = NativeMeasurementStateService::default();
    let current = process(10, 1, "/usr/local/bin/codex", 0.0);
    state.apply(
        group(&[current.clone()]),
        vec![current.clone()],
        Some(BTreeMap::from([(10, (10, 20))])),
        (100.0, 1.0),
    );
    let gap = state.apply(
        group(&[current.clone()]),
        vec![current.clone()],
        None,
        (102.0, 3.0),
    );
    assert!(gap[0]
        .metrics
        .as_ref()
        .unwrap()
        .network_status
        .starts_with("Unavailable"));
    let recovered = state.apply(
        group(&[current.clone()]),
        vec![current.clone()],
        Some(BTreeMap::from([(10, (1_000, 2_000))])),
        (104.0, 5.0),
    );
    assert_eq!(
        recovered[0].metrics.as_ref().unwrap().received_per_second,
        None
    );
    let resumed = state.apply(
        group(&[current.clone()]),
        vec![current],
        Some(BTreeMap::from([(10, (1_100, 2_400))])),
        (106.0, 7.0),
    );
    assert_eq!(
        resumed[0].metrics.as_ref().unwrap().received_per_second,
        Some(50.0)
    );
    assert_eq!(state.totals().received_bytes, 100);
}

#[test]
fn partial_network_and_missing_cpu_are_labeled_without_complete_totals() -> () {
    let mut state = NativeMeasurementStateService::default();
    let first = process(10, 1, "/usr/local/bin/codex", 0.0);
    let mut child = process(20, 10, "/bin/sh", 0.0);
    child.cpu_seconds = None;
    child.resident_bytes = None;
    state.apply(
        group(&[first.clone(), child.clone()]),
        vec![first.clone(), child.clone()],
        Some(BTreeMap::from([(10, (10, 20))])),
        (100.0, 1.0),
    );
    let groups = state.apply(
        group(&[first.clone(), child.clone()]),
        vec![first, child],
        Some(BTreeMap::from([(10, (20, 40))])),
        (102.0, 3.0),
    );
    let metrics = groups[0].metrics.as_ref().unwrap();
    assert_eq!(metrics.cpu_percent, None);
    assert_eq!(metrics.resident_bytes, None);
    assert_eq!(metrics.cpu_covered_processes, 1);
    assert_eq!(metrics.memory_covered_processes, 1);
    assert_eq!(metrics.network_covered_processes, 1);
    assert_eq!(metrics.received_per_second, Some(5.0));
    assert!(metrics.network_status.starts_with("Partial"));
    assert_eq!(state.totals().resident_bytes, 1_000);
}

#[test]
fn histories_are_bounded_and_preserve_missing_samples() -> () {
    let mut state = NativeMeasurementStateService::default();
    let current = process(10, 1, "/usr/local/bin/codex", 0.0);
    let first = state.apply(
        group(&[current.clone()]),
        vec![current.clone()],
        None,
        (100.0, 1.0),
    );
    assert_eq!(first[0].history[0][1], None);
    let latest = history_after_samples(&mut state, &current, first, 70);
    assert_eq!(latest[0].history.len(), 61);
    assert_eq!(latest[0].history.first().unwrap()[0], Some(118.0));
    assert_eq!(latest[0].history.last().unwrap()[0], Some(238.0));
    assert_eq!(latest[0].history.last().unwrap()[3], None);
}

fn history_after_samples(
    state: &mut NativeMeasurementStateService,
    current: &NativeProcess,
    mut latest: Vec<NativeGroup>,
    total_samples: usize,
) -> Vec<NativeGroup> {
    for sample in 1..total_samples {
        latest = state.apply(
            group(&[current.clone()]),
            vec![current.clone()],
            None,
            (100.0 + sample as f64 * 2.0, 1.0 + sample as f64 * 2.0),
        );
    }
    latest
}

#[test]
fn aggregate_totals_do_not_double_count_nested_roots() -> () {
    let mut state = NativeMeasurementStateService::default();
    let processes = vec![
        process(
            10,
            1,
            "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT",
            0.0,
        ),
        process(20, 10, "/usr/local/bin/codex", 0.0),
    ];
    state.apply(group(&processes), processes, None, (100.0, 1.0));
    assert_eq!(state.totals().resident_bytes, 2_000);
    assert_eq!(state.totals().samples, 1);
}

struct FixtureProcessReader {
    samples: VecDeque<Result<Vec<NativeProcess>, String>>,
}
impl ProcessReader for FixtureProcessReader {
    fn read(&mut self) -> Result<Vec<NativeProcess>, String> {
        self.samples.pop_front().unwrap()
    }
}
struct FixtureNetworkReader {
    samples: VecDeque<Result<BTreeMap<i32, (i64, i64)>, String>>,
}
impl NetworkReader for FixtureNetworkReader {
    fn read(&mut self) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        self.samples.pop_front().unwrap()
    }
}
struct FixtureClock {
    time: (f64, f64),
}
impl ObservationClock for FixtureClock {
    fn now(&self) -> (f64, f64) {
        self.time
    }
}

#[test]
fn native_factory_starts_without_fabricated_measurements() -> () {
    let clock: Arc<dyn ObservationClock> = Arc::new(FixtureClock { time: (100.0, 1.0) });
    let commands = NativeCommandCompositionService::new(Arc::clone(&clock));
    let source: Box<dyn MetricSource> =
        NativeMeasurementFactoryService::with_commands(clock, &commands);
    assert_eq!(source.totals().samples, 0);
    assert!(!source.network_available());
}

#[test]
fn injected_facade_reports_unavailable_network_and_real_process_presence() -> () {
    let current = process(10, 1, "/usr/local/bin/codex", 0.0);
    let reader = FixtureProcessReader {
        samples: VecDeque::from([Ok(vec![current])]),
    };
    let network = FixtureNetworkReader {
        samples: VecDeque::from([Err("denied".to_owned())]),
    };
    let clock = FixtureClock { time: (100.0, 1.0) };
    let mut sampler =
        ProcessMeasurementService::new(Box::new(reader), Box::new(network), Arc::new(clock));
    let groups = MetricSource::sample(&mut sampler).unwrap();
    assert!(!MetricSource::network_available(&sampler));
    assert_eq!(groups[0].family, "Codex");
    assert_eq!(groups[0].status, "process-present");
    let metrics = groups[0].metrics.as_ref().unwrap();
    assert_eq!(metrics.received_per_second, None);
    assert_eq!(metrics.sent_per_second, None);
    assert_eq!(metrics.network_covered_processes, 0);
    assert!(metrics.network_status.starts_with("Unavailable:"));
    assert_eq!(MetricSource::totals(&sampler).samples, 1);
}

#[test]
fn process_reader_errors_are_not_fabricated_empty_samples() -> () {
    let reader = FixtureProcessReader {
        samples: VecDeque::from([Err("permission denied".to_owned())]),
    };
    let network = FixtureNetworkReader {
        samples: VecDeque::from([Ok(BTreeMap::new())]),
    };
    let clock = FixtureClock { time: (100.0, 1.0) };
    let mut sampler =
        ProcessMeasurementService::new(Box::new(reader), Box::new(network), Arc::new(clock));
    assert_eq!(
        MetricSource::sample(&mut sampler).unwrap_err(),
        "permission denied"
    );
    assert_eq!(MetricSource::totals(&sampler).samples, 0);
    assert!(MetricSource::command_observations(&sampler).is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn live_mac_identity_cpu_memory_and_storage_probe() -> () {
    let mut before: libc::rusage = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut before) },
        0
    );
    let before_seconds = before.ru_utime.tv_sec as f64
        + before.ru_utime.tv_usec as f64 / 1_000_000.0
        + before.ru_stime.tv_sec as f64
        + before.ru_stime.tv_usec as f64 / 1_000_000.0;
    let mut reader = NativeProcessReaderService::new();
    let processes = ProcessReader::read(&mut reader).unwrap();
    let own = processes
        .iter()
        .find(|process: &&NativeProcess| -> bool { process.pid == std::process::id() as i32 })
        .unwrap();
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    let seconds = usage.ru_utime.tv_sec as f64
        + usage.ru_utime.tv_usec as f64 / 1_000_000.0
        + usage.ru_stime.tv_sec as f64
        + usage.ru_stime.tv_usec as f64 / 1_000_000.0;
    assert_eq!(result, 0);
    assert_eq!(own.user, unsafe { libc::getuid() } as i64);
    assert!(!own.started.is_empty());
    assert!(!own.executable.is_empty());
    assert!(own.resident_bytes.unwrap() > 0);
    assert!(own.disk_read_bytes.is_some());
    assert!(own.disk_written_bytes.is_some());
    assert!(own.cpu_seconds.unwrap() >= before_seconds - 0.001);
    assert!(own.cpu_seconds.unwrap() <= seconds + 0.001);
    println!(
        "live libproc: own PID {}, CPU {:.6}s; bracketed getrusage {:.6}..{:.6}s; RSS {} bytes; storage read/write {}/{} bytes; accessible same-user processes {}",
        own.pid,
        own.cpu_seconds.unwrap(),
        before_seconds,
        seconds,
        own.resident_bytes.unwrap(),
        own.disk_read_bytes.unwrap(),
        own.disk_written_bytes.unwrap(),
        processes.len()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn live_mac_network_availability_probe_is_bounded() -> () {
    let began = std::time::Instant::now();
    let result = NativeNetworkReaderService::new().read();
    assert!(began.elapsed().as_secs_f64() < 6.0);
    println!(
        "live nettop: {}, {:.3}s",
        network_probe_description(&result),
        began.elapsed().as_secs_f64()
    );
}

#[cfg(target_os = "macos")]
fn network_probe_description(result: &Result<BTreeMap<i32, (i64, i64)>, String>) -> String {
    match result {
        Ok(counters) => format!("available, {} per-PID counter rows", counters.len()),
        Err(error) => format!("explicitly unavailable ({error})"),
    }
}

#[test]
fn live_platform_facade_returns_real_groups_and_rolling_samples() -> () {
    let clock = Arc::new(SystemClockService::new());
    let mut sampler = ProcessMeasurementService::new(
        NativePlatformCompositionService::process_reader(),
        NativePlatformCompositionService::network_reader(),
        clock,
    );
    let first = MetricSource::sample(&mut sampler).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let second = MetricSource::sample(&mut sampler).unwrap();
    let first_ids: std::collections::BTreeSet<String> = first
        .iter()
        .map(|group: &NativeGroup| -> String { group.id.clone() })
        .collect();
    let stable: Vec<&NativeGroup> = second
        .iter()
        .filter(|group: &&NativeGroup| -> bool { first_ids.contains(&group.id) })
        .collect();
    let unique: std::collections::BTreeSet<i32> = second
        .iter()
        .flat_map(
            |group: &NativeGroup| -> std::iter::Copied<std::slice::Iter<'_, i32>> {
                group.members.iter().copied()
            },
        )
        .collect();
    let count: usize = second
        .iter()
        .map(|group: &NativeGroup| -> usize { group.members.len() })
        .sum();
    assert_eq!(count, unique.len());
    assert_eq!(MetricSource::totals(&sampler).samples, 2);
    assert!(stable
        .iter()
        .all(|group: &&NativeGroup| -> bool { group.history.len() == 2 }));
    assert!(stable
        .iter()
        .all(|group: &&NativeGroup| -> bool { group.history[1][0] > group.history[0][0] }));
    assert!(second.iter().all(|group: &NativeGroup| -> bool {
        group.status == "process-present"
            && group.task_status == "unavailable"
            && !group.instrumented
    }));
    println!(
        "live facade: {} first groups, {} second groups, {} stable histories, {} disjoint measured processes, RSS total {} bytes, network available {}",
        first.len(),
        second.len(),
        stable.len(),
        count,
        MetricSource::totals(&sampler).resident_bytes,
        MetricSource::network_available(&sampler)
    );
    let wire = serde_json::to_value(&second).unwrap();
    assert!(wire
        .as_array()
        .unwrap()
        .iter()
        .all(|group: &serde_json::Value| -> bool {
            group.get("tasks").is_none() && group.get("terminals").is_none()
        }));
}
