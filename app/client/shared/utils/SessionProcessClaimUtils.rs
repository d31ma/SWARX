use crate::models::{
    NativeCommandObservation, NativeProcessMetrics, NativeProcessOwner, NativeProcessTreeSample,
    NativeRuntimeView, NativeSessionProcessClaims, NativeSessionResourceView,
};
use crate::utils::{ProcessGraphUtils, ProcessOwnerUtils, ProcessTreeUtils};
use std::collections::{BTreeMap, BTreeSet};

pub struct SessionProcessClaimUtils;

impl SessionProcessClaimUtils {
    const UNIQUE: usize = 1;
    const HISTORY_SECONDS: f64 = 120.0;
    const HISTORY_SAMPLES: usize = 61;
    const MICROSECONDS_PER_SECOND: u32 = 1_000_000;

    pub fn resolve(sessions: &mut [NativeSessionResourceView]) -> () {
        let claims: NativeSessionProcessClaims = Self::claims(sessions);
        Self::project(sessions, &claims);
    }

    // Inputs are current, binding-verified command_resources. Resolve each historical tick
    // independently; these samples can only supply a chart, never the live process tree.
    pub fn history_samples(
        sources: &[(String, NativeCommandObservation)],
        now: f64,
    ) -> BTreeMap<String, Vec<NativeProcessTreeSample>> {
        let ticks = Self::history_ticks(sources, now);
        let mut result = Self::project_history(sources, ticks);
        Self::limit_history(&mut result);
        result
    }

    fn history_ticks<'a>(
        sources: &'a [(String, NativeCommandObservation)],
        now: f64,
    ) -> BTreeMap<u64, Vec<(&'a str, &'a NativeCommandObservation, &'a NativeProcessTreeSample)>> {
        let mut ticks = BTreeMap::new();
        for (session, observation) in sources {
            Self::observation_ticks(session, observation, now, &mut ticks);
        }
        ticks
    }

    fn observation_ticks<'a>(
        session: &'a str,
        observation: &'a NativeCommandObservation,
        now: f64,
        ticks: &mut BTreeMap<u64, Vec<(&'a str, &'a NativeCommandObservation, &'a NativeProcessTreeSample)>>,
    ) -> () {
        for sample in observation.tree_history.iter()
            .filter(|sample| Self::valid_history_sample(observation, sample, now)) {
            ticks.entry(sample.time.to_bits()).or_default().push((session, observation, sample));
        }
    }

    fn project_history(
        sources: &[(String, NativeCommandObservation)],
        ticks: BTreeMap<u64, Vec<(&str, &NativeCommandObservation, &NativeProcessTreeSample)>>,
    ) -> BTreeMap<String, Vec<NativeProcessTreeSample>> {
        let mut result = BTreeMap::new();
        for (time, samples) in ticks {
            let claims = Self::history_claims(sources, &samples);
            let mut processes = BTreeMap::new();
            let mut seen = BTreeMap::new();
            Self::history_members(&samples, &claims, (&mut processes, &mut seen));
            Self::append_tick(time, processes, &mut result);
        }
        result
    }

    fn history_members(
        samples: &[(&str, &NativeCommandObservation, &NativeProcessTreeSample)],
        claims: &NativeSessionProcessClaims,
        state: (&mut BTreeMap<String, Vec<NativeProcessMetrics>>,
            &mut BTreeMap<String, BTreeSet<(i32, String)>>),
    ) -> () {
        for (session, observation, sample) in samples {
            Self::history_sample((session, observation, sample), claims,
                (&mut *state.0, &mut *state.1));
        }
    }

    fn history_sample(
        source: (&str, &NativeCommandObservation, &NativeProcessTreeSample),
        claims: &NativeSessionProcessClaims,
        state: (&mut BTreeMap<String, Vec<NativeProcessMetrics>>,
            &mut BTreeMap<String, BTreeSet<(i32, String)>>),
    ) -> () {
        let (session, observation, sample) = source;
        // Preserve the empty tick even when an ambiguous root cannot contribute processes.
        let members = state.0.entry(session.to_owned()).or_default();
        let root = (observation.pid, observation.started.clone());
        if !Self::assigned(&root, session, claims) { return; }
        let unique = state.1.entry(session.to_owned()).or_default();
        Self::sample_members(&sample.processes, (&root, session), claims, (members, unique));
    }

    fn sample_members(
        processes: &[NativeProcessMetrics],
        command: (&(i32, String), &str),
        claims: &NativeSessionProcessClaims,
        state: (&mut Vec<NativeProcessMetrics>, &mut BTreeSet<(i32, String)>),
    ) -> () {
        for process in processes {
            let keep = Self::keep_process(process, command, claims, &mut *state.1);
            Self::append_history_member(process, keep, &mut *state.0);
        }
    }

    fn append_history_member(process: &NativeProcessMetrics, keep: bool, members: &mut Vec<NativeProcessMetrics>) -> () {
        if keep { members.push(process.clone()); }
    }

    fn append_tick(
        time: u64,
        processes: BTreeMap<String, Vec<NativeProcessMetrics>>,
        result: &mut BTreeMap<String, Vec<NativeProcessTreeSample>>,
    ) -> () {
        for (session, members) in processes {
            result.entry(session).or_default().push(NativeProcessTreeSample {
                time: f64::from_bits(time), processes: members,
            });
        }
    }

    fn limit_history(result: &mut BTreeMap<String, Vec<NativeProcessTreeSample>>) -> () {
        for samples in result.values_mut() {
            let remove = samples.len().saturating_sub(Self::HISTORY_SAMPLES);
            samples.drain(..remove);
        }
    }

    fn valid_history_sample(
        observation: &NativeCommandObservation,
        sample: &NativeProcessTreeSample,
        now: f64,
    ) -> bool {
        let root_count = sample.processes.iter().filter(|process| process.pid == observation.pid).count();
        let root_matches = sample.processes.iter().any(|process| {
            (process.pid, &process.started) == (observation.pid, &observation.started)
        });
        let clock = Self::history_clock(observation, sample, now);
        let root = Self::history_root(sample.processes.is_empty(), root_count, root_matches);
        let processes = Self::history_processes(&sample.processes);
        [clock, observation.pid > 0, Self::history_birth(&observation.started), root, processes]
            .into_iter().all(std::convert::identity)
    }

    fn history_clock(observation: &NativeCommandObservation, sample: &NativeProcessTreeSample, now: f64) -> bool {
        [now.is_finite(), sample.time.is_finite(), sample.time >= 0.0,
            sample.time >= now - Self::HISTORY_SECONDS, sample.time <= observation.sampled_at,
            sample.time <= now].into_iter().all(std::convert::identity)
    }

    fn history_root(empty: bool, count: usize, matches: bool) -> bool {
        if empty { return true; }
        [count == Self::UNIQUE, matches].into_iter().all(std::convert::identity)
    }

    fn history_processes(processes: &[NativeProcessMetrics]) -> bool {
        processes.iter().all(|process| Self::history_process(process))
    }

    fn history_process(process: &NativeProcessMetrics) -> bool {
        [process.pid > 0, Self::history_birth(&process.started)]
            .into_iter().all(std::convert::identity)
    }

    fn history_birth(value: &str) -> bool {
        value.split_once(':').is_some_and(|(seconds, micros)| {
            [seconds.parse::<u64>().is_ok(),
                micros.parse::<u32>().is_ok_and(|value| value < Self::MICROSECONDS_PER_SECOND)]
                .into_iter().all(std::convert::identity)
        })
    }

    fn history_claims(
        sources: &[(String, NativeCommandObservation)],
        samples: &[(&str, &NativeCommandObservation, &NativeProcessTreeSample)],
    ) -> NativeSessionProcessClaims {
        let mut claims = NativeSessionProcessClaims::default();
        Self::reserve_roots(sources, &mut claims);
        Self::reserve_samples(samples, &mut claims);
        claims
    }

    fn reserve_roots(sources: &[(String, NativeCommandObservation)], claims: &mut NativeSessionProcessClaims) -> () {
        // Missing samples cannot make contradictory current binding roots exclusive.
        for (session, observation) in sources {
            claims.births.entry(observation.pid).or_default().insert(observation.started.clone());
            Self::record_claim((observation.pid, observation.started.clone()), true, session, claims);
        }
    }

    fn reserve_samples(
        samples: &[(&str, &NativeCommandObservation, &NativeProcessTreeSample)],
        claims: &mut NativeSessionProcessClaims,
    ) -> () {
        for (session, observation, sample) in samples {
            Self::reserve_processes(session, observation, sample, claims);
        }
    }

    fn reserve_processes(
        session: &str,
        observation: &NativeCommandObservation,
        sample: &NativeProcessTreeSample,
        claims: &mut NativeSessionProcessClaims,
    ) -> () {
        let command: (i32, String) = (observation.pid, observation.started.clone());
        for process in &sample.processes {
            claims.births.entry(process.pid).or_default().insert(process.started.clone());
            let root = (process.pid, &process.started) == (observation.pid, &observation.started);
            let lineage: bool = Self::nearest_root_is(&command, process, &sample.processes, claims);
            Self::record_lineage_claim(Self::identity(process), (root, lineage), session, claims);
        }
    }

    // A descendant belongs to the closest verified root above it in this tree. A command bound
    // explicitly inside another session's harness keeps its own subtree; the outer root does not
    // also claim those processes and make them ambiguous. Unknown lineage keeps the claim.
    fn nearest_root_is(
        command: &(i32, String),
        process: &NativeProcessMetrics,
        processes: &[NativeProcessMetrics],
        claims: &NativeSessionProcessClaims,
    ) -> bool {
        Self::nearest_root(process, processes, claims)
            .map(|nearest: (i32, String)| -> bool { &nearest == command })
            .unwrap_or(true)
    }

    fn nearest_root(
        process: &NativeProcessMetrics,
        processes: &[NativeProcessMetrics],
        claims: &NativeSessionProcessClaims,
    ) -> Option<(i32, String)> {
        let mut parent: i32 = process.parent;
        // Bounded by the tree size so a malformed parent cycle cannot loop.
        for _ in 0..processes.len() {
            let found: &NativeProcessMetrics = processes
                .iter()
                .find(|candidate: &&NativeProcessMetrics| -> bool { candidate.pid == parent })?;
            let identity: (i32, String) = Self::identity(found);
            if claims.roots.contains_key(&identity) {
                return Some(identity);
            }
            parent = found.parent;
        }
        None
    }

    fn record_lineage_claim(
        identity: (i32, String),
        relation: (bool, bool),
        session: &str,
        claims: &mut NativeSessionProcessClaims,
    ) -> () {
        if [relation.0, relation.1].into_iter().any(std::convert::identity) {
            Self::record_claim(identity, relation.0, session, claims);
        }
    }

    fn claims(sessions: &[NativeSessionResourceView]) -> NativeSessionProcessClaims {
        let mut claims: NativeSessionProcessClaims = NativeSessionProcessClaims::default();
        // Every current root first, so each descendant can follow its nearest verified root.
        for session in sessions {
            Self::session_roots(session, &mut claims);
        }
        for session in sessions {
            Self::session_claims(session, &mut claims);
        }
        claims
    }

    fn session_roots(
        session: &NativeSessionResourceView,
        claims: &mut NativeSessionProcessClaims,
    ) -> () {
        for command in &session.commands {
            let root: (i32, String) = Self::root_identity(command);
            claims.births.entry(root.0).or_default().insert(root.1.clone());
            Self::record_claim(root, true, &session.key, claims);
        }
    }

    fn session_claims(
        session: &NativeSessionResourceView,
        claims: &mut NativeSessionProcessClaims,
    ) -> () {
        for command in &session.commands {
            Self::command_claims(command, &session.key, claims);
        }
    }

    fn command_claims(
        command: &NativeRuntimeView,
        session: &str,
        claims: &mut NativeSessionProcessClaims,
    ) -> () {
        for process in &command.group.processes {
            Self::process_claim(process, command, session, claims);
        }
    }

    fn process_claim(
        process: &NativeProcessMetrics,
        command: &NativeRuntimeView,
        session: &str,
        claims: &mut NativeSessionProcessClaims,
    ) -> () {
        let identity: (i32, String) = Self::identity(process);
        claims
            .births
            .entry(process.pid)
            .or_default()
            .insert(process.started.clone());
        let command_root: (i32, String) = Self::root_identity(command);
        let root: bool = identity == command_root;
        let lineage: bool =
            Self::nearest_root_is(&command_root, process, &command.group.processes, claims);
        Self::record_lineage_claim(identity, (root, lineage), session, claims);
    }

    fn record_claim(
        identity: (i32, String),
        root: bool,
        session: &str,
        claims: &mut NativeSessionProcessClaims,
    ) -> () {
        if root {
            claims
                .roots
                .entry(identity)
                .or_default()
                .insert(session.to_owned());
            return;
        }
        claims
            .descendants
            .entry(identity)
            .or_default()
            .insert(session.to_owned());
    }

    fn project(
        sessions: &mut [NativeSessionResourceView],
        claims: &NativeSessionProcessClaims,
    ) -> () {
        for session in sessions {
            Self::project_session(session, claims);
        }
    }

    fn project_session(
        session: &mut NativeSessionResourceView,
        claims: &NativeSessionProcessClaims,
    ) -> () {
        let key: &str = &session.key;
        let owners: Vec<NativeProcessOwner> = Self::reported_owners(&session.commands);
        let mut roots: BTreeSet<(i32, String)> = BTreeSet::new();
        session
            .commands
            .retain(|command: &NativeRuntimeView| -> bool {
                Self::keep_root(command, key, claims, &mut roots)
            });
        let mut seen: BTreeSet<(i32, String)> = BTreeSet::new();
        Self::project_commands(&mut session.commands, key, (claims, &owners), &mut seen);
    }

    fn reported_owners(commands: &[NativeRuntimeView]) -> Vec<NativeProcessOwner> {
        commands
            .iter()
            .flat_map(|command: &NativeRuntimeView| -> Vec<NativeProcessOwner> {
                command.group.owners.clone()
            })
            .collect()
    }

    fn keep_root(
        command: &NativeRuntimeView,
        session: &str,
        claims: &NativeSessionProcessClaims,
        seen: &mut BTreeSet<(i32, String)>,
    ) -> bool {
        let identity: (i32, String) = Self::root_identity(command);
        if !Self::assigned(&identity, session, claims) {
            return false;
        }
        seen.insert(identity)
    }

    fn project_commands(
        commands: &mut [NativeRuntimeView],
        session: &str,
        context: (&NativeSessionProcessClaims, &[NativeProcessOwner]),
        seen: &mut BTreeSet<(i32, String)>,
    ) -> () {
        for command in commands {
            Self::project_command(command, session, context, seen);
        }
    }

    fn project_command(
        command: &mut NativeRuntimeView,
        session: &str,
        context: (&NativeSessionProcessClaims, &[NativeProcessOwner]),
        seen: &mut BTreeSet<(i32, String)>,
    ) -> () {
        let root: (i32, String) = Self::root_identity(command);
        command
            .group
            .processes
            .retain(|process: &NativeProcessMetrics| -> bool {
                Self::keep_process(process, (&root, session), context.0, seen)
            });
        Self::rebuild(command, context.1);
    }

    fn keep_process(
        process: &NativeProcessMetrics,
        command: (&(i32, String), &str),
        claims: &NativeSessionProcessClaims,
        seen: &mut BTreeSet<(i32, String)>,
    ) -> bool {
        let identity: (i32, String) = Self::identity(process);
        let allowed: bool = Self::allowed(&identity, command, claims);
        Self::remember_allowed(identity, allowed, seen)
    }

    fn allowed(
        identity: &(i32, String),
        command: (&(i32, String), &str),
        claims: &NativeSessionProcessClaims,
    ) -> bool {
        [
            Self::assigned(identity, command.1, claims),
            Self::belongs_to_command(identity, command.0, claims),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn belongs_to_command(
        identity: &(i32, String),
        root: &(i32, String),
        claims: &NativeSessionProcessClaims,
    ) -> bool {
        if claims.roots.contains_key(identity) {
            return identity == root;
        }
        true
    }

    fn remember_allowed(
        identity: (i32, String),
        allowed: bool,
        seen: &mut BTreeSet<(i32, String)>,
    ) -> bool {
        if !allowed {
            return false;
        }
        seen.insert(identity)
    }

    fn assigned(
        identity: &(i32, String),
        session: &str,
        claims: &NativeSessionProcessClaims,
    ) -> bool {
        [
            Self::unique_birth(identity.0, claims),
            Self::owner(identity, claims) == Some(session),
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn unique_birth(pid: i32, claims: &NativeSessionProcessClaims) -> bool {
        claims
            .births
            .get(&pid)
            .is_some_and(|births: &BTreeSet<String>| -> bool { births.len() == Self::UNIQUE })
    }

    fn owner<'a>(
        identity: &(i32, String),
        claims: &'a NativeSessionProcessClaims,
    ) -> Option<&'a str> {
        let sessions: Option<&BTreeSet<String>> = claims
            .roots
            .get(identity)
            .or_else(|| -> Option<&BTreeSet<String>> { claims.descendants.get(identity) });
        sessions
            .filter(|sessions: &&BTreeSet<String>| -> bool { sessions.len() == Self::UNIQUE })
            .and_then(|sessions: &BTreeSet<String>| -> Option<&str> {
                sessions.first().map(String::as_str)
            })
    }

    fn identity(process: &NativeProcessMetrics) -> (i32, String) {
        (process.pid, process.started.clone())
    }

    fn root_identity(command: &NativeRuntimeView) -> (i32, String) {
        (command.group.pid, command.group.started.clone())
    }

    fn rebuild(command: &mut NativeRuntimeView, owners: &[NativeProcessOwner]) -> () {
        command.group.members = command
            .group
            .processes
            .iter()
            .map(|process: &NativeProcessMetrics| -> i32 { process.pid })
            .collect();
        command.group.tree = ProcessTreeUtils::build(&command.group.processes, command.group.pid);
        command.group.owners = Self::current_owners(&command.group.processes, owners);
        ProcessOwnerUtils::rows(&mut command.group.tree, &command.group.owners);
        command.group.graph = ProcessGraphUtils::build(&command.group.tree);
        command.process_count = command.group.processes.len() as i64;
    }

    fn current_owners(
        processes: &[NativeProcessMetrics],
        owners: &[NativeProcessOwner],
    ) -> Vec<NativeProcessOwner> {
        owners
            .iter()
            .filter(|owner: &&NativeProcessOwner| -> bool {
                Self::matches_member(owner, processes)
            })
            .cloned()
            .collect()
    }

    fn matches_member(owner: &NativeProcessOwner, processes: &[NativeProcessMetrics]) -> bool {
        processes
            .iter()
            .any(|process: &NativeProcessMetrics| -> bool {
                (owner.pid, &owner.started) == (process.pid, &process.started)
            })
    }
}
