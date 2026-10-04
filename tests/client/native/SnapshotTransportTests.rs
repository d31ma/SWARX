use crate::models::{
    NativeOverviewView, NativeSnapshot, NativeSnapshotPage, NativeSnapshotTransfer,
    NativeViewSelection,
};
use crate::TachyonRustServices::NativeSnapshotTransportService;

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
    const MAXIMUM_BYTES: i64 = 2_097_152;
    const PAGE_BYTES: usize = 24576;
    const SHORTEST_FULL_PAGE: usize = 24573;
    const MAXIMUM_PAGES: i64 = 86;
    const MARKER: &'static str = "transfer-text-marker";
    const WIDE_CHARACTER: char = '🟣';

    fn text_view(text: String) -> NativeOverviewView {
        NativeOverviewView {
            limitations: vec![text],
            ..Self::view()
        }
    }

    fn text_offset() -> usize {
        serde_json::to_string(&Self::text_view(Self::MARKER.to_owned()))
            .unwrap()
            .find(Self::MARKER)
            .unwrap()
    }

    fn misaligned_text(units: usize) -> String {
        let lead: String = "x".repeat(Self::SHORTEST_FULL_PAGE - Self::text_offset());
        let filler: String = "x".repeat(Self::SHORTEST_FULL_PAGE - Self::WIDE_CHARACTER.len_utf8());
        let unit: String = format!("{}{filler}", Self::WIDE_CHARACTER);
        lead + &unit.repeat(units)
    }

    fn tachyon_quote(text: &str) -> String {
        format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
    }

    fn tachyon_reply(page: &NativeSnapshotPage) -> String {
        format!(
            "{{\"value\":{{\"id\":{},\"index\":{},\"content\":{},\"error\":{}}}}}",
            i64::MIN,
            i64::MIN,
            Self::tachyon_quote(&page.content),
            Self::tachyon_quote(&page.error)
        )
    }

    fn largest_reply(
        transport: &NativeSnapshotTransportService,
        ticket: &NativeSnapshotTransfer,
    ) -> usize {
        (0..ticket.pages)
            .map(|index: i64| -> usize {
                Self::tachyon_reply(&transport.page(ticket.id, index)).len()
            })
            .max()
            .unwrap_or_default()
    }

    fn has_control_character(page: &NativeSnapshotPage) -> bool {
        page.content
            .chars()
            .any(|character: char| -> bool { character < ' ' })
    }

    fn only_quotes_and_backslashes(page: &NativeSnapshotPage) -> bool {
        page.content
            .bytes()
            .all(|byte: u8| -> bool { byte == b'"' || byte == b'\\' })
    }

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

#[test]
fn quote_and_backslash_pages_stay_under_the_reply_limit_after_tachyon_escaping() -> () {
    let source = SnapshotTransportTests::text_view("\"\\".repeat(520_000));
    let mut transport = NativeSnapshotTransportService::default();
    let ticket = transport.open_view(&source);
    let full_page = transport.page(ticket.id, 1);
    let largest = SnapshotTransportTests::largest_reply(&transport, &ticket);
    assert!(ticket.error.is_empty());
    assert!(ticket.bytes <= SnapshotTransportTests::MAXIMUM_BYTES);
    assert!(ticket.pages <= SnapshotTransportTests::MAXIMUM_PAGES);
    assert_eq!(full_page.content.len(), SnapshotTransportTests::PAGE_BYTES);
    assert!(SnapshotTransportTests::only_quotes_and_backslashes(
        &full_page
    ));
    assert!(!SnapshotTransportTests::has_control_character(&full_page));
    assert!(largest > 2 * SnapshotTransportTests::PAGE_BYTES);
    assert!(largest < SnapshotTransportTests::PAGE_WIRE_LIMIT);
}

#[test]
fn wide_characters_on_every_page_edge_still_fit_the_page_limit() -> () {
    let source = SnapshotTransportTests::text_view(SnapshotTransportTests::misaligned_text(84));
    let mut transport = NativeSnapshotTransportService::default();
    let ticket = transport.open_view(&source);
    assert!(ticket.error.is_empty());
    assert!(ticket.bytes <= SnapshotTransportTests::MAXIMUM_BYTES);
    assert_eq!(ticket.pages, SnapshotTransportTests::MAXIMUM_PAGES);
    assert_eq!(
        transport.page(ticket.id, 0).content.len(),
        SnapshotTransportTests::SHORTEST_FULL_PAGE
    );
    assert_eq!(
        transport.page(ticket.id, 1).content.len(),
        SnapshotTransportTests::SHORTEST_FULL_PAGE
    );
    assert!(
        SnapshotTransportTests::largest_reply(&transport, &ticket)
            < SnapshotTransportTests::PAGE_WIRE_LIMIT
    );
}
