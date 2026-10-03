use crate::models::{NativeGroup, NativeGroupMetrics, NativeRuntimeView};
use crate::utils::MetricFormatUtils;

pub struct RuntimeViewUtils;

impl RuntimeViewUtils {
    const UNAVAILABLE: &'static str = "Unavailable";
    const NO_ATTRIBUTION: &'static str = "No per-agent process attribution";
    const NO_SCOPE: &'static str = "No process attribution";
    const SHARED_SCOPE: &'static str = "shared-runtime";
    const TREE_SCOPE: &'static str = "local-process-tree";
    const COMMAND_SCOPE: &'static str = "reported-command-process";
    const PROGRESS_UNAVAILABLE: &'static str = "Progress unavailable";
    const PARTIAL: &'static str = "partial";
    const PARTIAL_NOTE: &'static str = "Partial counters";
    const STATE_PREFIX: &'static str = "state-";

    pub fn group(group: &NativeGroup) -> NativeRuntimeView {
        let mut view: NativeRuntimeView = Self::base(group);
        Self::metrics(&mut view, group.metrics.as_ref());
        view
    }

    fn base(group: &NativeGroup) -> NativeRuntimeView {
        NativeRuntimeView {
            group: group.clone(),
            charts: Vec::new(),
            selected: String::new(),
            provider_header: false,
            cpu: Self::UNAVAILABLE.to_owned(),
            memory: Self::UNAVAILABLE.to_owned(),
            download: Self::UNAVAILABLE.to_owned(),
            upload: Self::UNAVAILABLE.to_owned(),
            disk_read: Self::UNAVAILABLE.to_owned(),
            disk_write: Self::UNAVAILABLE.to_owned(),
            network_note: Self::NO_ATTRIBUTION.to_owned(),
            cpu_total: Self::UNAVAILABLE.to_owned(),
            download_total: Self::UNAVAILABLE.to_owned(),
            upload_total: Self::UNAVAILABLE.to_owned(),
            peak_memory: Self::UNAVAILABLE.to_owned(),
            scope_label: Self::scope(&group.scope),
            hook_label: Self::hook(group.instrumented),
            process_count: group.processes.len() as i64,
            progress: Self::PROGRESS_UNAVAILABLE.to_owned(),
            tasks: Vec::new(),
            terminals: Vec::new(),
            avatar: Self::avatar(&group.family),
            status_class: format!("{}{}", Self::STATE_PREFIX, group.status),
        }
    }

    fn metrics(view: &mut NativeRuntimeView, metrics: Option<&NativeGroupMetrics>) -> () {
        if let Some(metrics) = metrics {
            Self::measured(view, metrics);
        }
    }

    fn measured(view: &mut NativeRuntimeView, metrics: &NativeGroupMetrics) -> () {
        view.cpu = MetricFormatUtils::cpu(metrics.cpu_percent);
        view.memory = MetricFormatUtils::bytes(
            metrics
                .resident_bytes
                .map(|value: i64| -> f64 { value as f64 }),
        );
        view.download = MetricFormatUtils::rate(metrics.received_per_second);
        view.upload = MetricFormatUtils::rate(metrics.sent_per_second);
        view.disk_read = MetricFormatUtils::rate(metrics.disk_read_per_second);
        view.disk_write = MetricFormatUtils::rate(metrics.disk_write_per_second);
        view.network_note = Self::network(metrics);
        view.cpu_total = MetricFormatUtils::core_seconds(Some(metrics.totals.cpu_seconds));
        view.download_total = MetricFormatUtils::bytes(Some(metrics.totals.received_bytes as f64));
        view.upload_total = MetricFormatUtils::bytes(Some(metrics.totals.sent_bytes as f64));
        view.peak_memory =
            MetricFormatUtils::bytes(Some(metrics.totals.peak_resident_bytes as f64));
    }

    fn scope(scope: &str) -> String {
        match scope {
            Self::COMMAND_SCOPE => "Command root process".to_owned(),
            Self::SHARED_SCOPE => "Shared runtime".to_owned(),
            Self::TREE_SCOPE => "Process tree".to_owned(),
            _ => Self::NO_SCOPE.to_owned(),
        }
    }

    fn hook(connected: bool) -> String {
        if connected {
            return "Local hook connected".to_owned();
        }
        "No task hook".to_owned()
    }

    fn avatar(family: &str) -> String {
        match family {
            "Codex" => "CX".to_owned(),
            "Claude" => "CL".to_owned(),
            _ => "AG".to_owned(),
        }
    }

    fn network(metrics: &NativeGroupMetrics) -> String {
        match metrics.network_status.as_str() {
            Self::PARTIAL => format!(
                "{} \u{b7} {} processes",
                Self::PARTIAL_NOTE,
                metrics.network_covered_processes
            ),
            "baseline" => "Waiting for counter baseline".to_owned(),
            "no-counters" => "No visible socket counters".to_owned(),
            "unavailable" => "Network collection unavailable".to_owned(),
            _ => "Partial process counters".to_owned(),
        }
    }
}
