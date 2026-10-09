use crate::models::{
    NativeCommandBindingEvent, NativeCommandOwner, NativeExecutionRecord, NativeLogicalAgent,
    NativeLogicalEvent, NativeSessionLink, NativeTaskRecord,
};
use crate::utils::NativeCommandContextUtils;
use std::collections::BTreeMap;

#[derive(Default)]
#[Service]
pub struct LogicalStateService {
    agents: BTreeMap<String, NativeLogicalAgent>,
    revision: i64,
    text: LogicalTextService,
    policy: LogicalEventPolicyService,
    freshness: LogicalFreshnessService,
    coordination: LogicalCoordinationService,
}

impl LogicalStateService {
    const CAPACITY_REACHED: &'static str = "Logical agent capacity reached.";
    const UNFINISHED_TASK: &'static str = "Assign an unfinished task before starting an execution.";
    const NO_EXECUTION: &'static str = "No running execution with this ID.";
    const FIRST_OBSERVATION: f64 = 0.0;
    pub fn apply(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<i64, String> {
        let kind: String = self.text.identifier(Some(&event.event_type))?;
        let handler: fn(&mut Self, &NativeLogicalEvent, f64) -> Result<(), String> =
            Self::handler(&kind)?;
        self.require_time(now)?;
        let identity = self.require_event_identity(&kind, event)?;
        let previous = self.agents.get(&identity).cloned();
        handler(self, event, now)?;
        self.note_activity(&kind, &identity, previous.as_ref(), now);
        self.revision += 1;
        Ok(self.revision)
    }

    pub fn rows(&self, now: f64) -> Vec<NativeLogicalAgent> {
        self.fresh_rows(now)
    }

    pub fn revision(&self) -> i64 {
        self.revision
    }

    // The owner row comes from the current rows, live or recovered: after a restart a session's
    // main agent may only be a recovered row while its sub-agents send the hooks.
    pub fn command_event(
        &self,
        event: &NativeCommandBindingEvent,
        rows: &[NativeLogicalAgent],
    ) -> Result<NativeCommandBindingEvent, String> {
        let mut normalized: NativeCommandBindingEvent = event.clone();
        normalized.owner.agent_id = self.command_identity(&event.owner)?;
        let owner: Option<&NativeLogicalAgent> = rows
            .iter()
            .find(|row: &&NativeLogicalAgent| -> bool { row.id == normalized.owner.agent_id });
        NativeCommandContextUtils::require_session_root(owner, &normalized)?;
        Ok(normalized)
    }

    pub fn command_revision(&mut self) -> i64 {
        self.revision += 1;
        self.revision
    }

    fn command_identity(&self, owner: &NativeCommandOwner) -> Result<String, String> {
        if owner.agent_id.starts_with('/') {
            return self
                .coordination
                .identity(Some(&owner.session_id), &owner.agent_id);
        }
        self.text.identifier(Some(&owner.agent_id))
    }

    fn handler(
        kind: &str,
    ) -> Result<fn(&mut Self, &NativeLogicalEvent, f64) -> Result<(), String>, String> {
        match kind {
            "logical.register" => Ok(Self::register),
            "logical.task" => Ok(Self::task),
            "logical.execution.start" => Ok(Self::start_execution),
            "logical.execution.finish" => Ok(Self::finish_execution),
            "logical.status" => Ok(Self::status),
            "logical.heartbeat" => Ok(Self::heartbeat),
            "logical.session" => Ok(Self::session),
            LogicalCoordinationService::REGISTER => Ok(Self::coordination_register),
            LogicalCoordinationService::UPSERT => Ok(Self::coordination_upsert),
            LogicalCoordinationService::EXECUTION_START => Ok(Self::coordination_start),
            LogicalCoordinationService::EXECUTION_FINISH => Ok(Self::coordination_finish),
            _ => Err("Unsupported logical event.".to_string()),
        }
    }

    fn register(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        self.require_new_agent(&event.agent)?;
        let session: Option<NativeSessionLink> = self.session_binding(event)?;
        let agent: NativeLogicalAgent = NativeLogicalAgent {
            id: event.agent.clone(),
            name: self.text.name(event.name.as_deref(), &event.agent)?,
            family: self.text.name(event.family.as_deref(), "Agent")?,
            status: LogicalEventPolicyService::ACTIVE.to_string(),
            source: LogicalEventPolicyService::SOURCE.to_string(),
            captured_at: now,
            last_activity_at: None,
            feed_mode: LogicalEventPolicyService::MODE.to_string(),
            task: None,
            executions: Vec::new(),
            metrics: None,
            session,
            age_seconds: 0.0,
            stale: false,
            reported_status: None,
            model: self.text.model(event.model.as_deref()),
        };
        self.make_room(now);
        self.require_agent_capacity()?;
        self.agents.insert(event.agent.clone(), agent);
        Ok(())
    }

    fn coordination_register(
        &mut self,
        event: &NativeLogicalEvent,
        now: f64,
    ) -> Result<(), String> {
        let row: NativeLogicalAgent = self.coordination.row(event, now)?;
        self.require_new_agent(&row.id)?;
        self.make_room(now);
        self.require_agent_capacity()?;
        self.agents.insert(row.id.clone(), row);
        Ok(())
    }

    fn coordination_upsert(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        let mut row: NativeLogicalAgent = self.coordination.row(event, now)?;
        let exists: bool = self.agents.contains_key(&row.id);
        self.room_for(exists, now);
        let full: bool = Self::new_row_full(self.agents.len(), exists);
        Self::require_new_row_space(full)?;
        row.executions = self.coordination_executions(self.agents.get(&row.id), &row)?;
        self.agents.insert(row.id.clone(), row);
        Ok(())
    }

    fn coordination_start(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        let mapped: NativeLogicalEvent = self.coordination_event(event)?;
        let safe: String = self.text.tool(event.name.as_deref());
        Self::require_safe_action(event.name.as_deref(), &safe)?;
        let row: NativeLogicalAgent = self.agent(&mapped.agent)?;
        self.require_reported_room(&row.executions)?;
        self.start_execution(&mapped, now)
    }

    // A coordinator reports every finish explicitly, so its full list of running actions refuses
    // a new one rather than dropping a running action.
    fn require_reported_room(&self, values: &[NativeExecutionRecord]) -> Result<(), String> {
        let full: bool = values.len() >= LogicalEventPolicyService::EXECUTION_LIMIT;
        let settled: bool = values
            .iter()
            .any(|item: &NativeExecutionRecord| -> bool { self.policy.is_settled(&item.status) });
        if full && !settled {
            return Err("Running execution capacity reached.".to_string());
        }
        Ok(())
    }

    fn coordination_finish(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        let mapped: NativeLogicalEvent = self.coordination_event(event)?;
        self.finish_execution(&mapped, now)
    }

    fn coordination_event(&self, event: &NativeLogicalEvent) -> Result<NativeLogicalEvent, String> {
        let mut mapped: NativeLogicalEvent = event.clone();
        mapped.agent = self
            .coordination
            .identity(event.session_id.as_deref(), &event.agent)?;
        let requested: String = self.text.identifier(event.task.as_deref())?;
        let row: NativeLogicalAgent = self.agent(&mapped.agent)?;
        let task: &NativeTaskRecord = self.current_task(row.task.as_ref())?;
        self.require_matching_task(&requested, &task.id)?;
        Ok(mapped)
    }

    fn require_safe_action(name: Option<&str>, safe: &str) -> Result<(), String> {
        if name != Some(safe) {
            return Err("Report only a safe short tool identifier for the action.".to_string());
        }
        Ok(())
    }

    fn coordination_executions(
        &self,
        previous: Option<&NativeLogicalAgent>,
        next: &NativeLogicalAgent,
    ) -> Result<Vec<NativeExecutionRecord>, String> {
        match previous {
            None => Ok(Vec::new()),
            Some(previous) => self.retained_executions(previous, next),
        }
    }

    fn retained_executions(
        &self,
        previous: &NativeLogicalAgent,
        next: &NativeLogicalAgent,
    ) -> Result<Vec<NativeExecutionRecord>, String> {
        let running: bool = Self::has_running(&previous.executions);
        let previous_task: Option<&str> = previous
            .task
            .as_ref()
            .map(|task: &NativeTaskRecord| -> &str { task.id.as_str() });
        let next_task: Option<&str> = next
            .task
            .as_ref()
            .map(|task: &NativeTaskRecord| -> &str { task.id.as_str() });
        let same: bool = previous_task == next_task;
        let changing: bool = Self::changes_running(running, same);
        self.require_finished(changing)?;
        self.require_finished(self.finishes_running(&next.status, running))?;
        Ok(previous.executions.clone())
    }

    fn changes_running(running: bool, same: bool) -> bool {
        running && !same
    }

    fn new_row_full(count: usize, exists: bool) -> bool {
        count >= LogicalEventPolicyService::AGENT_LIMIT && !exists
    }

    fn require_new_row_space(full: bool) -> Result<(), String> {
        if full {
            return Err(Self::CAPACITY_REACHED.to_string());
        }
        Ok(())
    }

    fn require_event_identity(&self, kind: &str, event: &NativeLogicalEvent) -> Result<String, String> {
        match kind {
            LogicalCoordinationService::REGISTER
            | LogicalCoordinationService::UPSERT
            | LogicalCoordinationService::EXECUTION_START
            | LogicalCoordinationService::EXECUTION_FINISH => self
                .coordination
                .identity(event.session_id.as_deref(), &event.agent),
            _ => self
                .text
                .identifier(Some(&event.agent)),
        }
    }

    fn task(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        let mut agent: NativeLogicalAgent = self.agent(&event.agent)?;
        let identifier: String = self.text.identifier(event.task.as_deref())?;
        let status: String = self.policy.status(event.status.as_deref())?;
        let finishing: bool = self.policy.is_final(&status);
        let dependencies: Vec<String> = self.dependencies(event.depends_on.as_deref())?;
        let reported: Option<String> = self
            .text
            .title(event.task_name.as_deref())
            .or_else(|| -> Option<String> { event.name.clone() });
        let retained: String = Self::retained_name(agent.task.as_ref(), &identifier);
        let task: NativeTaskRecord = NativeTaskRecord {
            id: identifier.clone(),
            name: self.text.name(reported.as_deref(), &retained)?,
            status,
            reported_status: None,
            depends_on: dependencies,
        };
        // A running call keeps running only within its own unfinished assignment.
        Self::retire_running(
            &mut agent.executions,
            (!finishing).then_some(identifier.as_str()),
        );
        agent.task = Some(task);
        self.save_agent(agent, now)
    }

    /// Ends the wait for a finish that can no longer come: the reporter moved to another
    /// assignment, finished the assignment, or finished the agent without finishing these calls
    /// (Claude Code sends no finish for a denied or cancelled call). The record keeps no finish
    /// time, duration or exit code; a later finish with the same exact ID still applies.
    fn retire_running(executions: &mut [NativeExecutionRecord], kept: Option<&str>) -> () {
        executions
            .iter_mut()
            .filter(|item: &&mut NativeExecutionRecord| -> bool {
                item.status == LogicalEventPolicyService::RUNNING
                    && (kept.is_none() || item.task_id.as_deref() != kept)
            })
            .for_each(|item: &mut NativeExecutionRecord| -> () {
                item.status = LogicalEventPolicyService::NO_FINISH.to_string();
            });
    }

    fn retained_name(previous: Option<&NativeTaskRecord>, identifier: &str) -> String {
        previous
            .filter(|task: &&NativeTaskRecord| -> bool { task.id == identifier })
            .map(|task: &NativeTaskRecord| -> String { task.name.clone() })
            .unwrap_or_else(|| -> String { identifier.to_string() })
    }

    fn start_execution(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        let mut agent: NativeLogicalAgent = self.agent(&event.agent)?;
        let task: &NativeTaskRecord = self.current_task(agent.task.as_ref())?;
        self.require_unfinished(&task.status)?;
        let requested: &str = event.task.as_deref().unwrap_or(&task.id);
        self.require_matching_task(requested, &task.id)?;
        let identifier: String = self.text.identifier(event.execution.as_deref())?;
        self.require_new_execution(&agent.executions, &identifier)?;
        let evicted: Option<usize> = self.evictable(&agent.executions);
        self.evict(&mut agent.executions, evicted);
        agent.executions.push(NativeExecutionRecord {
            id: identifier,
            name: self.text.tool(event.name.as_deref()),
            status: LogicalEventPolicyService::RUNNING.to_string(),
            exit_code: None,
            duration_seconds: None,
            started_at: now,
            reported_status: None,
            task_id: Some(task.id.clone()),
            task_name: Some(task.name.clone()),
            session_id: agent
                .session
                .as_ref()
                .map(|session: &NativeSessionLink| -> String { session.id.clone() }),
            session_name: agent
                .session
                .as_ref()
                .map(|session: &NativeSessionLink| -> String { session.name.clone() }),
            source: Some(agent.source.clone()),
            finished_at: None,
            duration_source: None,
            detail: self.text.detail(event.detail.as_deref()),
            background: event.background,
            title: self.text.description(event.title.as_deref()),
        });
        self.save_agent(agent, now)
    }

    /// The record a full list gives up for a new start: the oldest settled foreground record,
    /// else the oldest settled one, else the oldest call still waiting for its finish. A new
    /// start is never refused for room, and no finish is invented to make it.
    fn evictable(&self, values: &[NativeExecutionRecord]) -> Option<usize> {
        let foreground: Option<usize> = values
            .iter()
            .position(|item: &NativeExecutionRecord| -> bool { self.settled_foreground(item) });
        foreground
            .or(values
                .iter()
                .position(|item: &NativeExecutionRecord| -> bool {
                    self.policy.is_settled(&item.status)
                }))
            .or((!values.is_empty()).then_some(0))
    }

    fn settled_foreground(&self, item: &NativeExecutionRecord) -> bool {
        [
            self.policy.is_settled(&item.status),
            item.background != Some(true),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn finish_execution(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        let mut agent: NativeLogicalAgent = self.agent(&event.agent)?;
        let identifier: String = self.text.identifier(event.execution.as_deref())?;
        let position: usize = self.execution_position(&agent.executions, &identifier)?;
        let item: &mut NativeExecutionRecord = &mut agent.executions[position];
        self.require_running(&item.status)?;
        let status: String = self.policy.final_status(event.status.as_deref())?;
        let exit_code: Option<i64> = self.policy.exit_code(event.exit_code)?;
        let duration: f64 = self
            .policy
            .duration(event.duration_seconds, now - item.started_at)?;
        item.status = status;
        item.exit_code = exit_code;
        item.duration_seconds = Some(duration);
        item.finished_at = Some(now);
        item.duration_source = Some(Self::duration_source(event.duration_seconds).to_string());
        self.save_agent(agent, now)
    }

    fn status(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        let mut agent: NativeLogicalAgent = self.agent(&event.agent)?;
        let status: String = self.policy.status(event.status.as_deref())?;
        if self.policy.is_final(&status) {
            Self::retire_running(&mut agent.executions, None);
        }
        agent.status = status;
        agent.model = self.text.model(event.model.as_deref()).or(agent.model);
        self.save_agent(agent, now)
    }

    fn note_activity(
        &mut self,
        kind: &str,
        identity: &str,
        previous: Option<&NativeLogicalAgent>,
        now: f64,
    ) -> () {
        let eligible = matches!(kind,
            "logical.task" | "logical.status" | "logical.execution.start"
                | "logical.execution.finish" | LogicalCoordinationService::REGISTER
                | LogicalCoordinationService::UPSERT | LogicalCoordinationService::EXECUTION_START
                | LogicalCoordinationService::EXECUTION_FINISH);
        if !eligible {
            return;
        }
        if let Some(next) = self.agents.get_mut(identity) {
            let task_changed = Self::task_activity(previous) != Self::task_activity(Some(next));
            let executions_changed = Self::execution_activity(previous) != Self::execution_activity(Some(next));
            let terminal = self.policy.is_final(&next.status)
                && previous.is_some_and(|old| old.status != next.status);
            next.last_activity_at = if task_changed || executions_changed || terminal {
                Some(now)
            } else {
                previous.and_then(|old| old.last_activity_at)
            };
        }
    }

    fn task_activity(agent: Option<&NativeLogicalAgent>) -> Option<(&str, &str)> {
        agent.and_then(|row| row.task.as_ref()).map(|task| (task.id.as_str(), task.status.as_str()))
    }

    fn execution_activity(agent: Option<&NativeLogicalAgent>) -> Vec<(&str, &str)> {
        let mut values: Vec<(&str, &str)> = agent.into_iter().flat_map(|row| &row.executions)
            .map(|item| (item.id.as_str(), item.status.as_str())).collect();
        values.sort();
        values
    }

    fn heartbeat(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        let agent: NativeLogicalAgent = self.agent(&event.agent)?;
        self.save_agent(agent, now)
    }

    fn session(&mut self, event: &NativeLogicalEvent, now: f64) -> Result<(), String> {
        let mut agent: NativeLogicalAgent = self.agent(&event.agent)?;
        agent.session = self.session_binding(event)?;
        self.save_agent(agent, now)
    }

    fn save_agent(&mut self, mut agent: NativeLogicalAgent, now: f64) -> Result<(), String> {
        agent.captured_at = now;
        self.agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    fn session_binding(
        &self,
        event: &NativeLogicalEvent,
    ) -> Result<Option<NativeSessionLink>, String> {
        event
            .session_id
            .as_deref()
            .map(|identifier: &str| -> Result<NativeSessionLink, String> {
                self.bound_session(identifier, event.session_name.as_deref())
            })
            .transpose()
    }

    fn bound_session(
        &self,
        identifier: &str,
        name: Option<&str>,
    ) -> Result<NativeSessionLink, String> {
        let id: String = self.text.identifier(Some(identifier))?;
        Ok(NativeSessionLink {
            name: self.text.name(name, &id)?,
            id,
            source: LogicalEventPolicyService::SOURCE.to_string(),
        })
    }

    fn agent(&self, identifier: &str) -> Result<NativeLogicalAgent, String> {
        self.agents
            .get(identifier)
            .cloned()
            .ok_or_else(|| -> String {
                "Register the logical agent before sending events.".to_string()
            })
    }

    fn require_new_agent(&self, identifier: &str) -> Result<(), String> {
        if self.agents.contains_key(identifier) {
            return Err(
                "Logical agent already registered; use a new ID for a new run.".to_string(),
            );
        }
        Ok(())
    }

    // The live table is bounded and rows are never removed otherwise, so a long run of sub-agents
    // would refuse every later agent (a first Codex session, a new Claude session). When full, the
    // row quiet longest gives way, but only past the stale window and with no running call.
    fn make_room(&mut self, now: f64) -> () {
        let victim: Option<String> = self.room_victim(now);
        self.remove_row(victim);
    }

    fn room_for(&mut self, exists: bool, now: f64) -> () {
        if exists {
            return;
        }
        self.make_room(now);
    }

    fn room_victim(&self, now: f64) -> Option<String> {
        let full: bool = self.agents.len() >= LogicalEventPolicyService::AGENT_LIMIT;
        full.then(|| -> Option<String> { self.quietest(now) }).flatten()
    }

    fn quietest(&self, now: f64) -> Option<String> {
        self.agents
            .values()
            .filter(|agent: &&NativeLogicalAgent| -> bool { Self::evictable_row(agent, now) })
            .min_by(|left: &&NativeLogicalAgent, right: &&NativeLogicalAgent| -> std::cmp::Ordering {
                left.captured_at.total_cmp(&right.captured_at)
            })
            .map(|agent: &NativeLogicalAgent| -> String { agent.id.clone() })
    }

    fn evictable_row(agent: &NativeLogicalAgent, now: f64) -> bool {
        [
            now - agent.captured_at > LogicalEventPolicyService::STALE_SECONDS,
            !agent.executions.iter().any(|execution: &NativeExecutionRecord| -> bool {
                execution.status == LogicalEventPolicyService::RUNNING
            }),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn remove_row(&mut self, victim: Option<String>) -> () {
        if let Some(identifier) = victim {
            self.agents.remove(&identifier);
        }
    }

    fn require_agent_capacity(&self) -> Result<(), String> {
        if self.agents.len() >= LogicalEventPolicyService::AGENT_LIMIT {
            return Err(Self::CAPACITY_REACHED.to_string());
        }
        Ok(())
    }

    fn current_task<'a>(
        &self,
        value: Option<&'a NativeTaskRecord>,
    ) -> Result<&'a NativeTaskRecord, String> {
        value.ok_or_else(|| -> String { Self::UNFINISHED_TASK.to_string() })
    }

    fn require_unfinished(&self, status: &str) -> Result<(), String> {
        if self.policy.is_final(status) {
            return Err(Self::UNFINISHED_TASK.to_string());
        }
        Ok(())
    }

    fn require_finished(&self, finishing: bool) -> Result<(), String> {
        if finishing {
            return Err(
                "Finish running executions before finishing the assignment or agent.".to_string(),
            );
        }
        Ok(())
    }

    fn has_running(executions: &[NativeExecutionRecord]) -> bool {
        executions
            .iter()
            .any(|item: &NativeExecutionRecord| -> bool {
                item.status == LogicalEventPolicyService::RUNNING
            })
    }

    fn finishes_running(&self, status: &str, running: bool) -> bool {
        self.policy.is_final(status) && running
    }

    fn duration_source(reported: Option<f64>) -> &'static str {
        match reported {
            Some(_) => "reported",
            None => "receipt-elapsed",
        }
    }

    fn require_matching_task(&self, requested: &str, current: &str) -> Result<(), String> {
        if requested != current {
            return Err("Execution task does not match the current assignment.".to_string());
        }
        Ok(())
    }

    fn require_new_execution(
        &self,
        values: &[NativeExecutionRecord],
        identifier: &str,
    ) -> Result<(), String> {
        if values
            .iter()
            .any(|item: &NativeExecutionRecord| -> bool { item.id == identifier })
        {
            return Err("Execution ID already used.".to_string());
        }
        Ok(())
    }

    fn evict(&self, values: &mut Vec<NativeExecutionRecord>, position: Option<usize>) -> () {
        if values.len() >= LogicalEventPolicyService::EXECUTION_LIMIT {
            values.remove(position.unwrap_or_default());
        }
    }

    fn execution_position(
        &self,
        values: &[NativeExecutionRecord],
        identifier: &str,
    ) -> Result<usize, String> {
        values
            .iter()
            .position(|item: &NativeExecutionRecord| -> bool { item.id == identifier })
            .ok_or_else(|| -> String { Self::NO_EXECUTION.to_string() })
    }

    /// A finish matches a running call, or a call retired without a finish, by its exact ID.
    fn require_running(&self, status: &str) -> Result<(), String> {
        let open: bool = [
            LogicalEventPolicyService::RUNNING,
            LogicalEventPolicyService::NO_FINISH,
        ]
        .contains(&status);
        if !open {
            return Err(Self::NO_EXECUTION.to_string());
        }
        Ok(())
    }

    fn dependencies(&self, values: Option<&[String]>) -> Result<Vec<String>, String> {
        let dependencies: &[String] = values.unwrap_or_default();
        self.require_dependency_capacity(dependencies.len())?;
        let mut result: Vec<String> = Vec::new();
        for dependency in dependencies {
            result.push(self.text.identifier(Some(dependency))?);
        }
        Ok(result)
    }

    fn require_dependency_capacity(&self, count: usize) -> Result<(), String> {
        if count > LogicalEventPolicyService::EXECUTION_LIMIT {
            return Err("Reported dependency capacity reached.".to_string());
        }
        Ok(())
    }

    fn require_time(&self, now: f64) -> Result<(), String> {
        let conditions: [bool; 2] = [now.is_finite(), now >= Self::FIRST_OBSERVATION];
        let valid: bool = conditions
            .into_iter()
            .all(|condition: bool| -> bool { condition });
        Self::require_valid_time(valid)
    }

    fn require_valid_time(valid: bool) -> Result<(), String> {
        if !valid {
            return Err("Invalid observation timestamp.".to_string());
        }
        Ok(())
    }

    fn fresh_rows(&self, now: f64) -> Vec<NativeLogicalAgent> {
        let mut result: Vec<NativeLogicalAgent> = Vec::new();
        for agent in self.agents.values() {
            result.push(self.freshness.row(agent, now));
        }
        result
    }
}
