use crate::models::{NativeActiveSessionView, NativeActiveSummaryView, NativeSummaryCardView};
use crate::utils::{ActiveSessionUtils, TimeFormatUtils};
use std::collections::BTreeSet;

pub struct ActiveSummaryUtils;

impl ActiveSummaryUtils {
    const LIVE: &'static str = "Live";
    const NOT_CURRENT_VALUE: &'static str = "\u{2014}";
    const NO_SAMPLE: &'static str = "Not current \u{b7} no sample yet";
    const LAST_SAMPLE: &'static str = "Not current \u{b7} last sample ";
    const NO_SAMPLE_TIME: f64 = 0.0;
    const NO_HARNESS: &'static str = "None reporting";
    const HARNESS_SEPARATOR: &'static str = ", ";
    const AGENTS: (&'static str, &'static str) = ("agents", "AGENTS WORKING");
    const SESSIONS: (&'static str, &'static str) = ("sessions", "ACTIVE SESSIONS");
    const TOOLS: (&'static str, &'static str) = ("tools", "TOOLS RUNNING");
    const HARNESSES: (&'static str, &'static str) = ("harnesses", "HARNESSES ACTIVE");
    const SESSION_DETAIL: &'static str = "Verified session IDs";
    const TOOL_DETAIL: &'static str = "Unfinished tool calls";
    const NONE: i64 = 0;

    pub fn summary(
        sessions: &[NativeActiveSessionView],
        live_status: &str,
        sampled_at: f64,
    ) -> NativeActiveSummaryView {
        let working: Vec<NativeActiveSessionView> = Self::working(sessions);
        let sessions: &[NativeActiveSessionView] = &working;
        let total: i64 = Self::total(sessions, Self::agent_total);
        let sub_agents: i64 = Self::total(sessions, Self::sub_agent_total);
        let tools: i64 = Self::total(sessions, Self::tool_total);
        let background: (i64, i64) = (
            Self::total(sessions, Self::running_total),
            Self::total(sessions, Self::waiting_total),
        );
        let harnesses: Vec<String> = Self::harnesses(sessions);
        let state: (bool, String) = (live_status == Self::LIVE, Self::notice(sampled_at));
        let roles: String = format!(
            "{} Agent \u{b7} {} Sub-agent",
            total - sub_agents,
            sub_agents
        );
        let cards: Vec<NativeSummaryCardView> = vec![
            Self::card(Self::AGENTS, total.to_string(), roles, &state),
            Self::card(
                Self::SESSIONS,
                sessions.len().to_string(),
                Self::session_detail(Self::unlinked_count(sessions)),
                &state,
            ),
            Self::card(
                Self::TOOLS,
                tools.to_string(),
                Self::tool_detail(background),
                &state,
            ),
            Self::card(
                Self::HARNESSES,
                harnesses.len().to_string(),
                Self::harness_detail(&harnesses),
                &state,
            ),
        ];
        NativeActiveSummaryView {
            current: state.0,
            agents: total,
            sub_agents,
            sessions: sessions.len() as i64,
            tools,
            background_running: background.0,
            background_waiting: background.1,
            harnesses,
            cards,
        }
    }

    fn working(sessions: &[NativeActiveSessionView]) -> Vec<NativeActiveSessionView> {
        sessions.iter().filter(|session| !session.idle).cloned().collect()
    }

    fn card(
        name: (&str, &str),
        value: String,
        detail: String,
        state: &(bool, String),
    ) -> NativeSummaryCardView {
        let (shown, explanation): (String, String) = Self::current_values(value, detail, state);
        NativeSummaryCardView {
            key: name.0.to_owned(),
            label: name.1.to_owned(),
            value: shown,
            detail: explanation,
        }
    }

    fn current_values(value: String, detail: String, state: &(bool, String)) -> (String, String) {
        if state.0 {
            return (value, detail);
        }
        (Self::NOT_CURRENT_VALUE.to_owned(), state.1.clone())
    }

    fn notice(sampled_at: f64) -> String {
        if sampled_at <= Self::NO_SAMPLE_TIME {
            return Self::NO_SAMPLE.to_owned();
        }
        format!(
            "{}{}",
            Self::LAST_SAMPLE,
            TimeFormatUtils::local_time(sampled_at)
        )
    }

    fn total(
        sessions: &[NativeActiveSessionView],
        count: fn(&NativeActiveSessionView) -> i64,
    ) -> i64 {
        sessions.iter().map(count).sum()
    }

    fn agent_total(session: &NativeActiveSessionView) -> i64 {
        session.agent_total
    }

    fn sub_agent_total(session: &NativeActiveSessionView) -> i64 {
        session.sub_agent_count
    }

    fn tool_total(session: &NativeActiveSessionView) -> i64 {
        session.tool_count
    }

    fn running_total(session: &NativeActiveSessionView) -> i64 {
        session.background_running
    }

    fn waiting_total(session: &NativeActiveSessionView) -> i64 {
        session.background_waiting
    }

    // Background tasks are task items, never agents: they add a labelled line to TOOLS RUNNING.
    fn tool_detail(background: (i64, i64)) -> String {
        let details: [String; 2] = [
            Self::TOOL_DETAIL.to_owned(),
            format!(
                "{} · {} background running · {} waiting",
                Self::TOOL_DETAIL,
                background.0,
                background.1
            ),
        ];
        details[usize::from(background.0 + background.1 > Self::NONE)].clone()
    }

    fn unlinked_count(sessions: &[NativeActiveSessionView]) -> i64 {
        sessions
            .iter()
            .filter(|session: &&NativeActiveSessionView| -> bool { !session.linked })
            .count() as i64
    }

    fn session_detail(unlinked: i64) -> String {
        if unlinked == Self::NONE {
            return Self::SESSION_DETAIL.to_owned();
        }
        format!("{unlinked} without a verified session ID")
    }

    fn harnesses(sessions: &[NativeActiveSessionView]) -> Vec<String> {
        sessions
            .iter()
            .map(|session: &NativeActiveSessionView| -> String {
                ActiveSessionUtils::family_harness(&session.family)
            })
            .collect::<BTreeSet<String>>()
            .into_iter()
            .collect()
    }

    fn harness_detail(harnesses: &[String]) -> String {
        if harnesses.is_empty() {
            return Self::NO_HARNESS.to_owned();
        }
        harnesses.join(Self::HARNESS_SEPARATOR)
    }
}
