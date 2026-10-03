use crate::models::{NativeOverviewView, NativeSnapshot, NativeSnapshotTransfer, NativeViewSelection};
use crate::TachyonRustServices::{NativeSnapshotTransportService};

struct OverviewTransportTests;

impl OverviewTransportTests {
    fn view() -> NativeOverviewView {
        crate::utils::OverviewViewUtils::view(&NativeSnapshot::default(), &NativeViewSelection::default(), 0.0)
    }

    fn content(
        transport: &NativeSnapshotTransportService,
        ticket: &NativeSnapshotTransfer,
    ) -> String {
        let mut content: String = String::new();
        for index in 0..ticket.pages {
            content.push_str(&transport.page(ticket.id, index).content);
        }
        content
    }
}

#[test]
fn overview_transfer_contains_only_the_projected_view_and_preserves_unicode() -> () {
    let mut transport: NativeSnapshotTransportService = NativeSnapshotTransportService::default();
    let mut view: NativeOverviewView = OverviewTransportTests::view();
    view.logical_notice = "Reported Unicode \u{2193} \u{2191} \u{b7} ".repeat(500);
    let ticket: NativeSnapshotTransfer = transport.open_view(&view);
    let content: String = OverviewTransportTests::content(&transport, &ticket);
    let decoded: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(ticket.error.is_empty());
    assert!(ticket.pages > 1);
    assert_eq!(ticket.bytes, content.len() as i64);
    assert_eq!(decoded["logicalNotice"], view.logical_notice);
    assert_eq!(decoded["liveStatus"], "Metrics unavailable");
    assert!(decoded.get("sampled_at").is_none());
    assert!(decoded.get("snapshot").is_none());
    assert!(transport.release(ticket.id));
    assert!(!transport.page(ticket.id, 0).error.is_empty());
}

#[test]
fn oversized_projected_view_stays_an_explicit_transfer_error() -> () {
    let mut transport: NativeSnapshotTransportService = NativeSnapshotTransportService::default();
    let mut view: NativeOverviewView = OverviewTransportTests::view();
    view.logical_notice = "x".repeat(2_097_153);
    let ticket: NativeSnapshotTransfer = transport.open_view(&view);
    assert_eq!(ticket.pages, 0);
    assert_eq!(ticket.bytes, 0);
    assert!(ticket.error.contains("2 MiB"));
}
