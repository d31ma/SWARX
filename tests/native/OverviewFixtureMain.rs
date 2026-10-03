use crate::OverviewFixtureRequest;
use crate::models::{
    NativeGroup, NativeLogicalAgent, NativeLogicalAgentView, NativeOverviewView, NativeRuntimeView,
    NativeSnapshot,
};
use crate::utils::{
    ChartDataUtils, CommandResourceViewUtils, LogicalViewUtils, OverviewViewUtils,
    ProcessGraphUtils, ProcessTreeUtils, ProcessOwnerUtils, RuntimeViewUtils, SessionViewUtils,
};
use std::io::Read;

pub struct OverviewFixtureMain;

impl OverviewFixtureMain {
    const MAXIMUM_INPUT_BYTES: u64 = 2_097_152;
    const PLATFORM: &'static str = "synthetic-macos-fixture";

    pub fn entry() -> i32 {
        let result: Result<(), String> = Self::run();
        Self::result(result)
    }

    fn run() -> Result<(), String> {
        let request: OverviewFixtureRequest = Self::read()?;
        let view: NativeOverviewView = Self::project(request);
        let output: String = serde_json::to_string(&view)
            .map_err(|error: serde_json::Error| -> String { error.to_string() })?;
        println!("{output}");
        Ok(())
    }

    fn read() -> Result<OverviewFixtureRequest, String> {
        let mut content: String = String::new();
        std::io::stdin()
            .take(Self::MAXIMUM_INPUT_BYTES)
            .read_to_string(&mut content)
            .map_err(|error: std::io::Error| -> String { error.to_string() })?;
        serde_json::from_str(&content)
            .map_err(|error: serde_json::Error| -> String { error.to_string() })
    }

    fn project(request: OverviewFixtureRequest) -> NativeOverviewView {
        let mut snapshot: NativeSnapshot = NativeSnapshot::default();
        snapshot.ready = true;
        snapshot.platform = Self::PLATFORM.to_owned();
        snapshot.sampled_at = request.sampled_at;
        snapshot.started_at = request.started_at;
        snapshot.totals = request.totals;
        snapshot.logical_views = Self::agents(&request.logical_agents, request.now);
        CommandResourceViewUtils::present(
            &mut snapshot.logical_views,
            &request.command_observations,
            &request.command_bindings,
            request.now,
        );
        let mut groups: Vec<NativeGroup> = request.groups;
        ProcessOwnerUtils::present(&mut groups, &mut snapshot.logical_views);
        snapshot.runtime_views = Self::groups(groups);
        ChartDataUtils::present(&mut snapshot.runtime_views, request.now);
        snapshot.session_groups = SessionViewUtils::sessions(&snapshot.logical_views);
        snapshot.linked_session_count = SessionViewUtils::linked_count(&snapshot.session_groups);
        snapshot.harness_count = SessionViewUtils::harness_count(&snapshot.session_groups);
        snapshot.activity = request.activity;
        snapshot.errors = request.errors;
        snapshot.limitations = request.limitations;
        OverviewViewUtils::view(&snapshot, &request.selection, request.now)
    }

    fn groups(groups: Vec<NativeGroup>) -> Vec<NativeRuntimeView> {
        let mut views: Vec<NativeRuntimeView> = Vec::new();
        for group in groups {
            views.push(Self::group(group));
        }
        views
    }

    fn group(mut group: NativeGroup) -> NativeRuntimeView {
        group.tree = ProcessTreeUtils::build(&group.processes, group.pid);
        ProcessOwnerUtils::rows(&mut group.tree, &group.owners);
        group.graph = ProcessGraphUtils::build(&group.tree);
        RuntimeViewUtils::group(&group)
    }

    fn agents(agents: &[NativeLogicalAgent], now: f64) -> Vec<NativeLogicalAgentView> {
        let mut views: Vec<NativeLogicalAgentView> = Vec::new();
        for agent in agents {
            views.push(LogicalViewUtils::agent(agent, now));
        }
        views
    }

    fn result(result: Result<(), String>) -> i32 {
        match result {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("{error}");
                1
            }
        }
    }
}
