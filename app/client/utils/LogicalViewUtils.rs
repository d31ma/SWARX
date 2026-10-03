use crate::models::{
    NativeCommandResourceView, NativeExecutionContext, NativeExecutionRecord, NativeExecutionView,
    NativeLogicalAgent, NativeLogicalAgentView, NativeSessionLink, NativeTaskRecord,
};
use crate::utils::{MetricFormatUtils, TimeFormatUtils};

pub struct LogicalViewUtils;

impl LogicalViewUtils {
    const SNAPSHOT: &'static str = "snapshot";
    const SNAPSHOT_SOURCE: &'static str = "Codex app-server thread/read snapshot";
    const RUNNING: &'static str = "running";
    const ACTIVE: &'static str = "active";
    const EVENTS: &'static str = "events";
    const COORDINATOR_REPORT: &'static str = "coordinator-report";
    const STATE_PREFIX: &'static str = "state-";
    const UNKNOWN_SOURCE: &'static str = "Unknown source";
    const NO_TASK: &'static str = "No task assigned by source";
    const UNAVAILABLE: &'static str = "Unavailable";
    const LAST_ACTION: &'static str = "LAST CAPTURED ACTION";
    const CURRENT_ACTION: &'static str = "CURRENT TOOL / ACTION";
    const ACTION_SEPARATOR: &'static str = ", ";
    const TIMING_SEPARATOR: &'static str = "; ";
    const ZERO: f64 = 0.0;
    const MISSING_DURATION: f64 = -1.0;

    pub fn agent(agent: &NativeLogicalAgent, now: f64) -> NativeLogicalAgentView {
        let mut view: NativeLogicalAgentView = Self::source_view(agent);
        Self::action_fields(&mut view, now);
        view
    }

    fn source_view(agent: &NativeLogicalAgent) -> NativeLogicalAgentView {
        NativeLogicalAgentView {
            id: agent.id.clone(),
            name: agent.name.clone(),
            family: agent.family.clone(),
            status: agent.status.clone(),
            source: agent.source.clone(),
            captured_at: agent.captured_at,
            feed_mode: agent.feed_mode.clone(),
            task: agent.task.clone(),
            executions: Self::executions(&agent.executions),
            metrics: agent.metrics,
            command_resources: Vec::new(),
            session: agent.session.clone(),
            age_seconds: agent.age_seconds,
            stale: agent.stale,
            reported_status: agent.reported_status.clone(),
            feed_label: Self::feed(&agent.feed_mode),
            task_label: Self::task_name(agent.task.as_ref()),
            task_state: Self::task_status(agent.task.as_ref()),
            captured_label: TimeFormatUtils::local_time(agent.captured_at),
            age_label: Self::age(agent),
            current_action: None,
            action_label: Self::CURRENT_ACTION.to_owned(),
            action_timing: None,
        }
    }

    fn action_fields(view: &mut NativeLogicalAgentView, now: f64) -> () {
        let captured: bool = Self::captured_mode(view);
        let current: Vec<NativeExecutionRecord> =
            Self::execution_records(&Self::current_views(view));
        let history: Vec<NativeExecutionRecord> = Self::execution_records(&view.executions);
        let action: Option<String> = Self::present(Self::current_actions(&current));
        let last: Option<String> = Self::last_action(&history);
        let timing: Option<String> = Self::present(Self::elapsed_times(&current, now));
        let completed: Option<String> = Self::completed_time(&history);
        view.current_action = Self::selected(captured, last, action);
        view.action_timing = Self::selected(captured, completed, timing);
        view.action_label = Self::action_label(captured);
    }

    pub fn running_view(agent: &NativeLogicalAgentView, now: f64) -> NativeLogicalAgentView {
        let mut view: NativeLogicalAgentView = agent.clone();
        Self::present_current(&mut view, now);
        view
    }

    fn present_current(view: &mut NativeLogicalAgentView, now: f64) -> () {
        view.executions = Self::current_views(view);
        let records: Vec<NativeExecutionRecord> = Self::execution_records(&view.executions);
        view.current_action = Self::present(Self::current_actions(&records));
        view.action_timing = Self::present(Self::elapsed_times(&records, now));
        view.action_label = Self::CURRENT_ACTION.to_owned();
        view.command_resources = Self::current_resources(view);
    }

    fn execution_records(views: &[NativeExecutionView]) -> Vec<NativeExecutionRecord> {
        views
            .iter()
            .map(|view: &NativeExecutionView| -> NativeExecutionRecord { view.execution.clone() })
            .collect()
    }

    fn current_views(agent: &NativeLogicalAgentView) -> Vec<NativeExecutionView> {
        agent
            .executions
            .iter()
            .filter(|view: &&NativeExecutionView| -> bool { Self::view_current(view, agent) })
            .cloned()
            .collect()
    }

    fn view_current(view: &NativeExecutionView, agent: &NativeLogicalAgentView) -> bool {
        let context: NativeExecutionContext = Self::context(
            agent.task.as_ref(),
            agent.session.as_ref(),
            &agent.source,
            Self::fresh_report(agent.stale, &agent.status, &agent.feed_mode, &agent.source),
        );
        Self::current_execution(&view.execution, &context)
    }

    fn fresh_report(stale: bool, status: &str, feed: &str, source: &str) -> bool {
        [
            !stale,
            [Self::ACTIVE, Self::RUNNING].contains(&status),
            [Self::EVENTS, Self::COORDINATOR_REPORT].contains(&feed),
            !source.trim().is_empty(),
            source != Self::SNAPSHOT_SOURCE,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn current_execution(
        execution: &NativeExecutionRecord,
        context: &NativeExecutionContext,
    ) -> bool {
        [
            context.fresh,
            Self::unfinished_command(execution),
            Self::matching_assignment(execution, context),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn context(
        task: Option<&NativeTaskRecord>,
        session: Option<&NativeSessionLink>,
        source: &str,
        fresh: bool,
    ) -> NativeExecutionContext {
        NativeExecutionContext {
            task: task.cloned(),
            session: session.cloned(),
            source: source.to_owned(),
            fresh,
        }
    }

    fn unfinished_command(execution: &NativeExecutionRecord) -> bool {
        [
            execution.status == Self::RUNNING,
            execution.finished_at.is_none(),
            execution.exit_code.is_none(),
            execution.duration_seconds.is_none(),
            !execution.id.trim().is_empty(),
            !execution.name.trim().is_empty(),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn matching_assignment(
        execution: &NativeExecutionRecord,
        context: &NativeExecutionContext,
    ) -> bool {
        [
            context.task.as_ref().is_some_and(Self::active_task),
            context.session.as_ref().is_some_and(Self::reported_session),
            context.task.as_ref().map(Self::task_identifier) == execution.task_id.as_deref(),
            context.session.as_ref().map(Self::session_identifier)
                == execution.session_id.as_deref(),
            execution.source.as_deref() == Some(context.source.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn active_task(task: &NativeTaskRecord) -> bool {
        [
            !task.id.trim().is_empty(),
            [Self::ACTIVE, Self::RUNNING].contains(&task.status.as_str()),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn reported_session(session: &NativeSessionLink) -> bool {
        !session.id.trim().is_empty()
    }

    fn task_identifier(task: &NativeTaskRecord) -> &str {
        task.id.as_str()
    }

    fn session_identifier(session: &NativeSessionLink) -> &str {
        session.id.as_str()
    }

    fn current_resources(agent: &NativeLogicalAgentView) -> Vec<NativeCommandResourceView> {
        agent
            .command_resources
            .iter()
            .filter(|resource: &&NativeCommandResourceView| -> bool {
                Self::matching_resource(resource, agent)
            })
            .cloned()
            .collect()
    }

    fn matching_resource(
        resource: &NativeCommandResourceView,
        agent: &NativeLogicalAgentView,
    ) -> bool {
        let owner: &crate::models::NativeCommandOwner = &resource.observation.owner;
        [
            owner.agent_id == agent.id,
            agent.task.as_ref().map(Self::task_identifier) == Some(owner.task_id.as_str()),
            agent.session.as_ref().map(Self::session_identifier) == Some(owner.session_id.as_str()),
            Self::has_execution(&agent.executions, &owner.execution_id),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn has_execution(views: &[NativeExecutionView], identifier: &str) -> bool {
        views
            .iter()
            .any(|view: &NativeExecutionView| -> bool { view.execution.id == identifier })
    }

    fn captured_mode(agent: &NativeLogicalAgentView) -> bool {
        [
            agent.feed_mode == Self::SNAPSHOT,
            agent.source == Self::SNAPSHOT_SOURCE,
        ]
        .into_iter()
        .any(std::convert::identity)
    }

    fn running(execution: &NativeExecutionRecord) -> bool {
        [Self::RUNNING, Self::ACTIVE].contains(&execution.status.as_str())
    }

    fn current_actions(executions: &[NativeExecutionRecord]) -> String {
        let mut names: Vec<String> = Vec::new();
        for execution in executions {
            Self::add_current(execution, &mut names);
        }
        names.join(Self::ACTION_SEPARATOR)
    }

    fn add_current(execution: &NativeExecutionRecord, names: &mut Vec<String>) -> () {
        if Self::running(execution) {
            names.push(execution.name.clone());
        }
    }

    fn present(value: String) -> Option<String> {
        if value.is_empty() {
            return None;
        }
        Some(value)
    }

    fn selected(captured: bool, last: Option<String>, fresh: Option<String>) -> Option<String> {
        if captured {
            return last;
        }
        fresh
    }

    fn last_action(executions: &[NativeExecutionRecord]) -> Option<String> {
        executions
            .last()
            .map(|execution: &NativeExecutionRecord| -> String { execution.name.clone() })
    }

    fn action_label(captured: bool) -> String {
        if captured {
            return Self::LAST_ACTION.to_owned();
        }
        Self::CURRENT_ACTION.to_owned()
    }

    fn elapsed_times(executions: &[NativeExecutionRecord], now: f64) -> String {
        let mut times: Vec<String> = Vec::new();
        for execution in executions {
            Self::add_elapsed(execution, now, &mut times);
        }
        times.join(Self::TIMING_SEPARATOR)
    }

    fn has_elapsed(execution: &NativeExecutionRecord, now: f64) -> bool {
        [
            Self::running(execution),
            execution.started_at > Self::ZERO,
            execution.started_at <= now,
            now.is_finite(),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn add_elapsed(execution: &NativeExecutionRecord, now: f64, times: &mut Vec<String>) -> () {
        if Self::has_elapsed(execution, now) {
            let elapsed: String = MetricFormatUtils::elapsed(now - execution.started_at);
            times.push(format!("{} \u{b7} elapsed {elapsed}", execution.name));
        }
    }

    fn completed_time(executions: &[NativeExecutionRecord]) -> Option<String> {
        let mut completed: Option<String> = None;
        for execution in executions {
            let candidate: Option<String> = Self::completed_candidate(execution);
            Self::replace_completed(candidate, &mut completed);
        }
        completed
    }

    fn completed_candidate(execution: &NativeExecutionRecord) -> Option<String> {
        let duration: f64 = execution.duration_seconds.unwrap_or(Self::MISSING_DURATION);
        let valid: bool = [
            !Self::running(execution),
            duration >= Self::ZERO,
            duration.is_finite(),
        ]
        .into_iter()
        .all(std::convert::identity);
        Self::completed_label(execution, duration, valid)
    }

    fn completed_label(
        execution: &NativeExecutionRecord,
        duration: f64,
        valid: bool,
    ) -> Option<String> {
        if !valid {
            return None;
        }
        let elapsed: String = MetricFormatUtils::elapsed(duration);
        Some(format!(
            "Last finished action: {} \u{b7} duration {elapsed}",
            execution.name
        ))
    }

    fn replace_completed(candidate: Option<String>, completed: &mut Option<String>) -> () {
        if candidate.is_some() {
            *completed = candidate;
        }
    }

    fn feed(mode: &str) -> String {
        match mode {
            Self::SNAPSHOT => "Captured snapshot".to_owned(),
            Self::EVENTS => "Explicit lifecycle events".to_owned(),
            "recovered" => "Recovered local roster".to_owned(),
            Self::COORDINATOR_REPORT => "Coordinator-reported roster".to_owned(),
            _ => Self::UNKNOWN_SOURCE.to_owned(),
        }
    }

    fn task_name(task: Option<&NativeTaskRecord>) -> String {
        task.map(|task: &NativeTaskRecord| -> String { task.name.clone() })
            .unwrap_or_else(|| -> String { Self::NO_TASK.to_owned() })
    }

    fn task_status(task: Option<&NativeTaskRecord>) -> String {
        task.map(|task: &NativeTaskRecord| -> String { task.status.clone() })
            .unwrap_or_else(|| -> String { Self::UNAVAILABLE.to_owned() })
    }

    fn age(agent: &NativeLogicalAgent) -> String {
        let elapsed: String = MetricFormatUtils::elapsed(agent.age_seconds);
        if agent.stale {
            return format!("Stale \u{b7} last report {elapsed} ago");
        }
        format!("{elapsed} since source update")
    }

    fn executions(executions: &[NativeExecutionRecord]) -> Vec<NativeExecutionView> {
        let mut views: Vec<NativeExecutionView> = Vec::new();
        for execution in executions {
            views.push(Self::execution(execution));
        }
        views
    }

    fn execution(execution: &NativeExecutionRecord) -> NativeExecutionView {
        NativeExecutionView {
            execution: execution.clone(),
            result: Self::exit(execution.exit_code),
            status_class: format!("{}{}", Self::STATE_PREFIX, execution.status),
        }
    }

    fn exit(code: Option<i64>) -> String {
        match code {
            Some(code) => format!("Exit {code}"),
            None => "Exit unavailable".to_owned(),
        }
    }
}
