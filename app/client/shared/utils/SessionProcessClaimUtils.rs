use crate::models::{
    NativeProcessMetrics, NativeProcessOwner, NativeRuntimeView, NativeSessionProcessClaims,
    NativeSessionResourceView,
};
use crate::utils::{ProcessGraphUtils, ProcessOwnerUtils, ProcessTreeUtils};
use std::collections::BTreeSet;

pub struct SessionProcessClaimUtils;

impl SessionProcessClaimUtils {
    const UNIQUE: usize = 1;

    pub fn resolve(sessions: &mut [NativeSessionResourceView]) -> () {
        let claims: NativeSessionProcessClaims = Self::claims(sessions);
        Self::project(sessions, &claims);
    }

    fn claims(sessions: &[NativeSessionResourceView]) -> NativeSessionProcessClaims {
        let mut claims: NativeSessionProcessClaims = NativeSessionProcessClaims::default();
        for session in sessions {
            Self::session_claims(session, &mut claims);
        }
        claims
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
        let root: bool = identity == Self::root_identity(command);
        Self::record_claim(identity, root, session, claims);
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
