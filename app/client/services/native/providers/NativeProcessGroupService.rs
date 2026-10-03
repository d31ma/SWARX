use crate::models::{NativeGroup, NativeProcess, NativeProviderSpec};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
#[Service]
pub struct NativeProcessGroupService {
    providers: NativeProviderService,
}

impl NativeProcessGroupService {
    const SHARED_SCOPE: &'static str = "shared-runtime";
    const LOCAL_SCOPE: &'static str = "local-process-tree";

    pub fn group(&self, processes: &[NativeProcess]) -> Vec<NativeGroup> {
        let by_pid: BTreeMap<i32, &NativeProcess> = self.by_pid(processes);
        let roots: Vec<(&NativeProcess, NativeProviderSpec)> = self.roots(processes, &by_pid);
        self.partition(roots, &by_pid)
    }

    fn by_pid<'a>(&self, processes: &'a [NativeProcess]) -> BTreeMap<i32, &'a NativeProcess> {
        processes
            .iter()
            .map(|process: &'a NativeProcess| -> (i32, &'a NativeProcess) {
                (process.pid, process)
            })
            .collect()
    }

    fn roots<'a>(
        &self,
        processes: &'a [NativeProcess],
        by_pid: &BTreeMap<i32, &NativeProcess>,
    ) -> Vec<(&'a NativeProcess, NativeProviderSpec)> {
        let mut roots: Vec<(&NativeProcess, NativeProviderSpec)> = Vec::new();
        for process in processes {
            self.add_root(process, &mut roots);
        }
        roots.sort_by_key(|(process, provider): &(&'a NativeProcess, NativeProviderSpec)| -> (bool, std::cmp::Reverse<usize>, i32) {
            (
                provider.shared,
                std::cmp::Reverse(self.depth(process.pid, by_pid)),
                process.pid,
            )
        });
        roots
    }

    fn add_root<'a>(
        &self,
        process: &'a NativeProcess,
        roots: &mut Vec<(&'a NativeProcess, NativeProviderSpec)>,
    ) -> () {
        if let Some(provider) = self.providers.classify(process) {
            roots.push((process, provider));
        }
    }

    fn depth(&self, identifier: i32, by_pid: &BTreeMap<i32, &NativeProcess>) -> usize {
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        let mut current: Option<i32> = Some(identifier);
        while let Some(identifier) = current {
            current = self.parent(identifier, by_pid, &mut seen);
        }
        seen.len()
    }

    fn parent(
        &self,
        identifier: i32,
        by_pid: &BTreeMap<i32, &NativeProcess>,
        seen: &mut BTreeSet<i32>,
    ) -> Option<i32> {
        if !seen.insert(identifier) {
            return None;
        }
        by_pid
            .get(&identifier)
            .map(|process: &&NativeProcess| -> i32 { process.parent })
    }

    fn partition(
        &self,
        roots: Vec<(&NativeProcess, NativeProviderSpec)>,
        by_pid: &BTreeMap<i32, &NativeProcess>,
    ) -> Vec<NativeGroup> {
        let mut claimed: BTreeSet<i32> = BTreeSet::new();
        let mut groups: Vec<NativeGroup> = Vec::new();
        for (process, provider) in roots {
            let descendants: BTreeSet<i32> = self.descendants(process.pid, by_pid);
            let members: Vec<i32> = descendants.difference(&claimed).copied().collect();
            self.add_group(process, provider, members, (&mut groups, &mut claimed));
        }
        groups
    }

    fn descendants(&self, root: i32, by_pid: &BTreeMap<i32, &NativeProcess>) -> BTreeSet<i32> {
        let mut stack: Vec<i32> = vec![root];
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        while let Some(identifier) = stack.pop() {
            self.visit(identifier, by_pid, &mut stack, &mut seen);
        }
        seen
    }

    fn visit(
        &self,
        identifier: i32,
        by_pid: &BTreeMap<i32, &NativeProcess>,
        stack: &mut Vec<i32>,
        seen: &mut BTreeSet<i32>,
    ) -> () {
        if !seen.insert(identifier) {
            return;
        }
        self.children(identifier, by_pid, stack);
    }

    fn children(
        &self,
        parent: i32,
        by_pid: &BTreeMap<i32, &NativeProcess>,
        stack: &mut Vec<i32>,
    ) -> () {
        for process in by_pid.values() {
            self.add_child(parent, process, stack);
        }
    }

    fn add_child(&self, parent: i32, process: &NativeProcess, stack: &mut Vec<i32>) -> () {
        if process.parent == parent {
            stack.push(process.pid);
        }
    }

    fn add_group(
        &self,
        process: &NativeProcess,
        provider: NativeProviderSpec,
        members: Vec<i32>,
        result: (&mut Vec<NativeGroup>, &mut BTreeSet<i32>),
    ) -> () {
        if members.is_empty() {
            return;
        }
        result.1.extend(members.iter().copied());
        result.0.push(self.make_group(process, provider, members));
    }

    fn make_group(
        &self,
        process: &NativeProcess,
        provider: NativeProviderSpec,
        members: Vec<i32>,
    ) -> NativeGroup {
        let suffix: &str = self.name_suffix(provider.shared);
        let scope: String = self.scope(provider.shared);
        NativeGroup {
            id: format!("{}:{}:{}", provider.id, process.pid, process.started),
            pid: process.pid,
            started: process.started.clone(),
            name: format!("{} {}", provider.family, suffix),
            family: provider.family,
            scope,
            status: "process-present".to_owned(),
            task_status: "unavailable".to_owned(),
            shared: provider.shared,
            instrumented: false,
            identity_evidence: provider.evidence,
            members,
            ..NativeGroup::default()
        }
    }

    fn name_suffix(&self, shared: bool) -> &'static str {
        if shared {
            return "desktop runtime";
        }
        "CLI process tree"
    }

    fn scope(&self, shared: bool) -> String {
        if shared {
            return Self::SHARED_SCOPE.to_owned();
        }
        Self::LOCAL_SCOPE.to_owned()
    }
}
