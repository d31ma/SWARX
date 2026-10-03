use crate::TachyonRustServices::NativeSnapshotTransportService;
use crate::models::{NativeOverviewView, NativeSnapshot, NativeSnapshotTransfer, NativeViewSelection};

struct SnapshotTransportTests;

impl SnapshotTransportTests {
    fn view() -> NativeOverviewView {
        crate::utils::OverviewViewUtils::view(
            &NativeSnapshot::default(),
            &NativeViewSelection::default(),
            0.0,
        )
    }

    const PAGE_WIRE_LIMIT: usize = 65536;

    fn collect_pages(
        transport: &NativeSnapshotTransportService,
        ticket: &NativeSnapshotTransfer,
    ) -> String {
        let mut content: String = String::new();
        for index in 0..ticket.pages {
            Self::append_page(transport, ticket.id, index, &mut content);
        }
        content
    }

    fn append_page(
        transport: &NativeSnapshotTransportService,
        id: i64,
        index: i64,
        content: &mut String,
    ) -> () {
        let page = transport.page(id, index);
        assert_eq!(page.id, id);
        assert_eq!(page.index, index);
        assert!(page.error.is_empty());
        assert!(serde_json::to_vec(&page).unwrap().len() < Self::PAGE_WIRE_LIMIT);
        content.push_str(&page.content);
    }
}

#[test]
fn large_unicode_snapshot_round_trips_in_bounded_pages() -> () {
    let source = NativeOverviewView {
        ready: true,
        sampled: "12:34:56".to_owned(),
        limitations: vec!["quoted \"text\"\nwith UTF-8 λ 🟣 ".repeat(8000)],
        ..SnapshotTransportTests::view()
    };
    let mut transport = NativeSnapshotTransportService::default();
    let ticket = transport.open_view(&source);
    assert!(ticket.error.is_empty());
    assert!(ticket.bytes > SnapshotTransportTests::PAGE_WIRE_LIMIT as i64);
    let content: String = SnapshotTransportTests::collect_pages(&transport, &ticket);
    assert_eq!(content.len() as i64, ticket.bytes);
    let restored: NativeOverviewView = serde_json::from_str(&content).unwrap();
    assert_eq!(restored.limitations, source.limitations);
    assert_eq!(restored.sampled, source.sampled);
    assert!(transport.release(ticket.id));
    assert!(!transport.page(ticket.id, 0).error.is_empty());
}

#[test]
fn opening_another_transfer_expires_old_pages_and_never_mixes_data() -> () {
    let mut transport = NativeSnapshotTransportService::default();
    let first = transport.open_view(&SnapshotTransportTests::view());
    let second = transport.open_view(&NativeOverviewView {
        sampled: "00:00:02".to_owned(),
        ..SnapshotTransportTests::view()
    });
    assert_ne!(first.id, second.id);
    assert!(!transport.page(first.id, 0).error.is_empty());
    assert!(!transport.release(first.id));
    assert!(transport.page(second.id, 0).error.is_empty());
    assert!(!transport.page(second.id, -1).error.is_empty());
    assert!(!transport.page(second.id, second.pages).error.is_empty());
}

#[test]
fn oversized_snapshots_fail_clearly_without_partial_data() -> () {
    let mut transport = NativeSnapshotTransportService::default();
    let source = NativeOverviewView {
        limitations: vec!["a".repeat(2 * 1024 * 1024)],
        ..SnapshotTransportTests::view()
    };
    let ticket = transport.open_view(&source);
    assert!(!ticket.error.is_empty());
    assert_eq!(ticket.pages, 0);
    assert_eq!(ticket.bytes, 0);
    assert!(!transport.page(ticket.id, 0).error.is_empty());
}

#[test]
fn smallest_signed_page_index_fails_without_panic_or_content() -> () {
    let mut transport = NativeSnapshotTransportService::default();
    let ticket = transport.open_view(&SnapshotTransportTests::view());
    let page = transport.page(ticket.id, i64::MIN);
    assert!(!page.error.is_empty());
    assert!(page.content.is_empty());
    assert!(transport.page(ticket.id, 0).error.is_empty());
}

#[test]
fn largest_signed_page_index_fails_without_overflow_or_content() -> () {
    let mut transport = NativeSnapshotTransportService::default();
    let ticket = transport.open_view(&SnapshotTransportTests::view());
    let page = transport.page(ticket.id, i64::MAX);
    assert!(!page.error.is_empty());
    assert!(page.content.is_empty());
    assert!(transport.page(ticket.id, 0).error.is_empty());
}

#[test]
fn retiring_the_page_refuses_its_transfer_and_preserves_fresh_generation() -> () {
    let mut transport: NativeSnapshotTransportService = NativeSnapshotTransportService::default();
    let retired: NativeSnapshotTransfer = transport.open_view(&SnapshotTransportTests::view());
    assert!(transport.page(retired.id, 0).error.is_empty());
    transport.retire();
    assert!(!transport.page(retired.id, 0).error.is_empty());
    assert!(!transport.release(retired.id));
    let current: NativeSnapshotTransfer = transport.open_view(&SnapshotTransportTests::view());
    assert_ne!(retired.id, current.id);
    assert!(transport.page(current.id, 0).error.is_empty());
    assert!(!transport.page(retired.id, 0).error.is_empty());
}
