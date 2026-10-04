// Fixture tests for the portable /proc text parser; they run on macOS and Linux.
use observatory_linux_collector_tests::models::NativeKernelProcess;
use observatory_linux_collector_tests::KernelProcessTextService;

fn text() -> KernelProcessTextService {
    KernelProcessTextService
}

fn stat_line(command: &str) -> String {
    format!(
        "1234 ({command}) S 1000 1234 1234 34816 1234 4194304 1500 3000 0 2 55 23 4 1 20 0 1 0 98765 12345678 900 18446744073709551615\n"
    )
}

fn expected() -> NativeKernelProcess {
    NativeKernelProcess {
        pid: 1234,
        parent: 1000,
        cpu_ticks: 78,
        start_ticks: 98765,
    }
}

#[test]
fn stat_line_reads_identity_and_cpu_ticks() -> () {
    assert_eq!(text().kernel_process(&stat_line("bash")), Some(expected()));
}

#[test]
fn stat_command_with_spaces_and_parentheses_is_skipped_whole() -> () {
    assert_eq!(
        text().kernel_process(&stat_line("my (odd) name")),
        Some(expected())
    );
    assert_eq!(
        text().kernel_process(&stat_line("x) S 9 9 9 9")),
        Some(expected())
    );
    assert_eq!(text().kernel_process(&stat_line(")")), Some(expected()));
    assert_eq!(text().kernel_process(&stat_line("")), Some(expected()));
}

#[test]
fn stat_cpu_ticks_are_user_plus_system_only() -> () {
    let line: &str = "7 (worker) R 1 7 7 0 -1 4194560 0 0 0 0 1000 250 999 999 20 0 4 0 42 0 0\n";
    let process: NativeKernelProcess = text().kernel_process(line).unwrap();
    assert_eq!(process.cpu_ticks, 1250);
    assert_eq!(process.start_ticks, 42);
    assert_eq!(process.parent, 1);
}

#[test]
fn malformed_or_truncated_stat_lines_are_rejected() -> () {
    assert_eq!(text().kernel_process(""), None);
    assert_eq!(text().kernel_process("1234 bash S 1000"), None);
    assert_eq!(text().kernel_process("1234 (bash) S 1000 1234 1234"), None);
    assert_eq!(
        text().kernel_process(&stat_line("bash").replacen("1234", "self", 1)),
        None
    );
    assert_eq!(
        text().kernel_process(&stat_line("bash").replacen(" 55 ", " x ", 1)),
        None
    );
}

#[test]
fn status_reports_the_real_user() -> () {
    let status: &str = "Name:\tbash\nUmask:\t0022\nState:\tS (sleeping)\nTgid:\t1234\nPid:\t1234\nPPid:\t1000\nUid:\t1001\t1002\t1003\t1004\nGid:\t20\t20\t20\t20\n";
    assert_eq!(text().real_user(status), Some(1001));
    assert_eq!(
        text().real_user("Name:\tbash\nGid:\t20\t20\t20\t20\n"),
        None
    );
    assert_eq!(text().real_user("Uid:\n"), None);
}

#[test]
fn statm_reports_resident_pages() -> () {
    assert_eq!(
        text().resident_pages("2048 512 300 20 0 400 0\n"),
        Some(512)
    );
    assert_eq!(text().resident_pages("2048\n"), None);
    assert_eq!(text().resident_pages(""), None);
}

#[test]
fn resident_bytes_multiply_pages_by_page_size() -> () {
    assert_eq!(text().resident_bytes(512, 4096), Some(2_097_152));
    assert_eq!(text().resident_bytes(512, 16384), Some(8_388_608));
    assert_eq!(text().resident_bytes(512, 0), None);
    assert_eq!(text().resident_bytes(512, -1), None);
    assert_eq!(text().resident_bytes(u64::MAX, 4096), None);
}

#[test]
fn io_reports_storage_bytes_and_missing_lines_stay_unavailable() -> () {
    let io: &str = "rchar: 100\nwchar: 200\nsyscr: 3\nsyscw: 4\nread_bytes: 4096\nwrite_bytes: 8192\ncancelled_write_bytes: 512\n";
    assert_eq!(text().disk_bytes(io), (Some(4096), Some(8192)));
    let reordered: &str = "cancelled_write_bytes: 512\nwrite_bytes: 8192\nread_bytes: 4096\n";
    assert_eq!(text().disk_bytes(reordered), (Some(4096), Some(8192)));
    assert_eq!(
        text().disk_bytes("rchar: 100\nread_bytes: 4096\n"),
        (Some(4096), None)
    );
    assert_eq!(text().disk_bytes("rchar: 100\nwchar: 200\n"), (None, None));
    assert_eq!(text().disk_bytes(""), (None, None));
}

#[test]
fn boot_id_is_trimmed_and_empty_is_unavailable() -> () {
    assert_eq!(
        text().boot("  6a1f0c2e-1111-4222-8333-944455556666\n"),
        Some("6a1f0c2e-1111-4222-8333-944455556666".to_owned())
    );
    assert_eq!(text().boot("\n"), None);
}

#[test]
fn proc_stat_btime_line_gives_boot_seconds() -> () {
    let stat: &str = "cpu  4705 356 584 3699 23 23 0 0 0 0\ncpu0 1393280 32966 572056 13343292 6130 0 17875 0 0 0\nintr 114930548 113199788 3 0 5 263 0 4 [...]\nctxt 1990473\nbtime 1062191376\nprocesses 2915\nprocs_running 1\nprocs_blocked 0\nsoftirq 229245889 94 60001584 13619 5175704 2471304 28 51212741 59130143 0 51240672\n";
    assert_eq!(text().boot_seconds(stat), Some(1_062_191_376));
    assert_eq!(text().boot_seconds("cpu 1 2 3\nctxt 9\n"), None);
    assert_eq!(text().boot_seconds("btime\n"), None);
    assert_eq!(text().boot_seconds("btime -5\n"), None);
    assert_eq!(text().boot_seconds("btimes 7\n"), None);
    assert_eq!(text().boot_seconds(""), None);
}

#[test]
fn start_ticks_become_epoch_seconds_and_microseconds() -> () {
    let boot: u64 = 1_700_000_000;
    assert_eq!(
        text().started(boot, 98765, 100),
        Some("1700000987:650000".to_owned())
    );
    assert_eq!(
        text().started(boot, 0, 100),
        Some("1700000000:0".to_owned())
    );
    assert_eq!(
        text().started(boot, 101, 100),
        Some("1700000001:10000".to_owned())
    );
    assert_eq!(
        text().started(boot, 2_501, 250),
        Some("1700000010:4000".to_owned())
    );
    assert_eq!(
        text().started(boot, 7, 1_000_000),
        Some("1700000000:7".to_owned())
    );
}

#[test]
fn start_identity_needs_a_positive_clock_rate_and_no_overflow() -> () {
    assert_eq!(text().started(1_700_000_000, 98765, 0), None);
    assert_eq!(text().started(1_700_000_000, 98765, -1), None);
    assert_eq!(text().started(u64::MAX, 100, 100), None);
}

#[test]
fn start_identity_matches_the_shared_birth_format() -> () {
    let started: String = text().started(1_700_000_000, 12_345, 100).unwrap();
    let (seconds, microseconds): (&str, &str) = started.split_once(':').unwrap();
    let seconds: u64 = seconds.parse().unwrap();
    let microseconds: u64 = microseconds.parse().unwrap();
    assert!(microseconds < 1_000_000);
    assert_eq!(format!("{seconds}:{microseconds}"), started);
    assert_eq!((seconds, microseconds), (1_700_000_123, 450_000));
}

#[test]
fn only_positive_numeric_entries_are_process_identifiers() -> () {
    assert_eq!(text().identifier("123"), Some(123));
    assert_eq!(text().identifier("1"), Some(1));
    assert_eq!(text().identifier("self"), None);
    assert_eq!(text().identifier("+5"), None);
    assert_eq!(text().identifier("0"), None);
    assert_eq!(text().identifier(""), None);
    assert_eq!(text().identifier("99999999999"), None);
}

#[test]
fn cpu_seconds_divide_ticks_by_the_clock_rate() -> () {
    assert_eq!(text().cpu_seconds(250, 100), Some(2.5));
    assert_eq!(text().cpu_seconds(0, 100), Some(0.0));
    assert_eq!(text().cpu_seconds(250, 0), None);
    assert_eq!(text().cpu_seconds(250, -1), None);
}
