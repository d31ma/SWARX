use observatory_windows_collector_tests::WindowsProcessConversionService;

fn values() -> WindowsProcessConversionService {
    WindowsProcessConversionService
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

const UNIX_EPOCH_TICKS: u64 = 116_444_736_000_000_000;

#[test]
fn file_time_halves_join_into_one_tick_count() -> () {
    assert_eq!(values().ticks(0, 0), 0);
    assert_eq!(values().ticks(7, 0), 7);
    assert_eq!(values().ticks(0, 1), 4_294_967_296);
    assert_eq!(values().ticks(u32::MAX, u32::MAX), u64::MAX);
    assert_eq!(
        values().ticks(0x89AB_CDEF, 0x0123_4567),
        0x0123_4567_89AB_CDEF
    );
}

#[test]
fn creation_time_becomes_unix_seconds_and_microseconds() -> () {
    let creation: u64 = UNIX_EPOCH_TICKS + 1_700_000_000 * 10_000_000;
    assert_eq!(values().started(creation).unwrap(), "1700000000:0");
    assert_eq!(
        values().started(UNIX_EPOCH_TICKS + 15_002_507).unwrap(),
        "1:500250"
    );
    assert_eq!(values().started(UNIX_EPOCH_TICKS).unwrap(), "0:0");
}

#[test]
fn creation_before_the_unix_epoch_is_unavailable() -> () {
    assert_eq!(values().started(UNIX_EPOCH_TICKS - 1), None);
    assert_eq!(values().started(0), None);
}

#[test]
fn start_identity_keeps_the_seconds_and_microseconds_shape() -> () {
    let started: String = values().started(UNIX_EPOCH_TICKS + 99_999_999).unwrap();
    let (seconds, microseconds): (&str, &str) = started.split_once(':').unwrap();
    assert_eq!(seconds.parse::<u64>().unwrap(), 9);
    assert_eq!(microseconds.parse::<u32>().unwrap(), 999_999);
}

#[test]
fn cpu_seconds_add_kernel_and_user_ticks() -> () {
    assert_eq!(values().cpu_seconds(25_000_000, 5_000_000), Some(3.0));
    assert_eq!(values().cpu_seconds(0, 0), Some(0.0));
    assert_eq!(values().cpu_seconds(1, 0), Some(0.0000001));
    assert_eq!(values().cpu_seconds(u64::MAX, 1), None);
}

#[test]
fn counters_outside_the_signed_range_are_unavailable() -> () {
    assert_eq!(values().counter(0), Some(0));
    assert_eq!(values().counter(4096), Some(4096));
    assert_eq!(values().counter(i64::MAX as u64), Some(i64::MAX));
    assert_eq!(values().counter(i64::MAX as u64 + 1), None);
    assert_eq!(values().counter(u64::MAX), None);
}

#[test]
fn only_positive_process_identifiers_are_listed() -> () {
    assert_eq!(values().identifier(4), Some(4));
    assert_eq!(values().identifier(1), Some(1));
    assert_eq!(values().identifier(0), None);
    assert_eq!(values().identifier(u32::MAX), None);
}

#[test]
fn parent_identifiers_keep_zero_and_reject_overflow() -> () {
    assert_eq!(values().parent(0), Some(0));
    assert_eq!(values().parent(1234), Some(1234));
    assert_eq!(values().parent(u32::MAX), None);
}

#[test]
fn executable_reads_exactly_the_reported_length() -> () {
    let mut buffer: Vec<u16> = wide("C:\\Users\\agent\\bin\\tool.exe");
    let length: u32 = buffer.len() as u32;
    buffer.extend([0, 0, 0]);
    assert_eq!(
        values().executable(&buffer, length).unwrap(),
        "C:\\Users\\agent\\bin\\tool.exe"
    );
    assert_eq!(values().executable(&buffer, 2).unwrap(), "C:");
}

#[test]
fn empty_overlong_or_invalid_executables_are_unavailable() -> () {
    let buffer: Vec<u16> = wide("C:\\tool.exe");
    assert_eq!(values().executable(&buffer, 0), None);
    assert_eq!(values().executable(&buffer, buffer.len() as u32 + 1), None);
    assert_eq!(values().executable(&[0xD800, 0x0041], 2), None);
}
