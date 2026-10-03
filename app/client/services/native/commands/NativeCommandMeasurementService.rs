use crate::models::{
    NativeCommandBinding, NativeCommandObservation, NativeCommandSample, NativeProcess,
    NativeProcessMetrics, NativeResourceTotals,
};
use crate::utils::ProcessLineageUtils;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

#[Service]
pub struct NativeCommandMeasurementService {
    registry: Option<Arc<Mutex<NativeCommandBindingRegistryService>>>,
    captured: Vec<NativeCommandBinding>,
    states: BTreeMap<String, NativeCommandSample>,
    latest: Vec<NativeCommandObservation>,
    counter: NativeCounterService,
}

impl NativeCommandMeasurementService {
    const HISTORY_SAMPLES: usize = 61;
    const HISTORY_SECONDS: f64 = 120.0;
    const AVAILABLE: &'static str =
        "Local root process only; missing/reset rates stay unavailable; network unavailable.";
    const GAP: &'static str = "Process sampling unavailable; missing resources are not zero.";
    const STATE_ERROR: &'static str = "Command binding state unavailable.";
    const UNIQUE_PROCESS: usize = 1;

    pub fn new(registry: Result<Arc<Mutex<NativeCommandBindingRegistryService>>, String>) -> Self {
        Self {
            registry: registry.ok(),
            captured: Vec::new(),
            states: BTreeMap::new(),
            latest: Vec::new(),
            counter: NativeCounterService::default(),
        }
    }

    pub fn capture(&mut self, time: (f64, f64)) -> () {
        self.captured = self.active(time).unwrap_or_default();
    }

    pub fn apply(&mut self, processes: &[NativeProcess], time: (f64, f64)) -> () {
        let current: Vec<NativeCommandBinding> = self.active(time).unwrap_or_default();
        let captured: Vec<NativeCommandBinding> = self.captured.clone();
        self.latest = self.measure_bindings(&captured, &current, processes, time);
        self.retain_current(time);
    }

    pub fn fail(&mut self, time: (f64, f64)) -> () {
        let current: Vec<NativeCommandBinding> = self.active(time).unwrap_or_default();
        let captured: Vec<NativeCommandBinding> = self.captured.clone();
        self.latest = self.missing_bindings(&captured, &current, time);
        self.retain_current(time);
    }

    pub fn observations(&self) -> Vec<NativeCommandObservation> {
        self.latest.clone()
    }

    pub fn attach_descendants(
        &mut self,
        processes: &[NativeProcess],
        measured: &BTreeMap<i32, NativeProcessMetrics>,
    ) -> () {
        Self::descendants(&mut self.latest, processes, measured);
    }

    fn descendants(
        observations: &mut [NativeCommandObservation],
        processes: &[NativeProcess],
        measured: &BTreeMap<i32, NativeProcessMetrics>,
    ) -> () {
        for observation in observations {
            observation.descendants = Self::observed_children(observation, processes, measured);
        }
    }

    fn observed_children(
        observation: &NativeCommandObservation,
        processes: &[NativeProcess],
        measured: &BTreeMap<i32, NativeProcessMetrics>,
    ) -> Vec<NativeProcessMetrics> {
        processes
            .iter()
            .find(|process: &&NativeProcess| -> bool {
                (process.pid, &process.started) == (observation.pid, &observation.started)
            })
            .map(|root: &NativeProcess| -> Vec<NativeProcessMetrics> {
                ProcessLineageUtils::descendants(root, processes, measured)
            })
            .unwrap_or_default()
    }

    fn active(&self, time: (f64, f64)) -> Result<Vec<NativeCommandBinding>, String> {
        let registry = self
            .registry
            .as_ref()
            .ok_or_else(|| -> String { Self::STATE_ERROR.to_owned() })?;
        registry
            .lock()
            .map_err(
                |_: std::sync::PoisonError<
                    std::sync::MutexGuard<'_, NativeCommandBindingRegistryService>,
                >|
                 -> String { Self::STATE_ERROR.to_owned() },
            )?
            .active(time)
    }

    fn measure_bindings(
        &mut self,
        captured: &[NativeCommandBinding],
        current: &[NativeCommandBinding],
        processes: &[NativeProcess],
        time: (f64, f64),
    ) -> Vec<NativeCommandObservation> {
        let mut result: Vec<NativeCommandObservation> = Vec::new();
        for binding in captured {
            let selected: Option<&NativeCommandBinding> = Self::current(binding, current);
            let process: Option<&NativeProcess> = Self::selected_process(selected, processes);
            self.revoke_missing(
                selected.filter(|_: &&NativeCommandBinding| -> bool { process.is_none() }),
            );
            self.append_measured(selected.zip(process), time, &mut result);
        }
        result
    }

    fn current<'a>(
        binding: &NativeCommandBinding,
        current: &'a [NativeCommandBinding],
    ) -> Option<&'a NativeCommandBinding> {
        current
            .iter()
            .find(|candidate: &&'a NativeCommandBinding| -> bool {
                [
                    binding.id == candidate.id,
                    binding.generation == candidate.generation,
                    binding.owner == candidate.owner,
                    binding.identity == candidate.identity,
                ]
                .into_iter()
                .all(std::convert::identity)
            })
    }

    fn append_measured(
        &mut self,
        selected: Option<(&NativeCommandBinding, &NativeProcess)>,
        time: (f64, f64),
        result: &mut Vec<NativeCommandObservation>,
    ) -> () {
        if let Some((binding, process)) = selected {
            result.push(self.measured(binding, process, time));
        }
    }

    fn same_process(binding: &NativeCommandBinding, process: &NativeProcess) -> bool {
        [
            binding.identity.pid == process.pid,
            binding.identity.started == process.started,
            binding.identity.user == process.user,
            binding.identity.executable == process.executable,
        ]
        .into_iter()
        .all(std::convert::identity)
    }

    fn selected_process<'a>(
        binding: Option<&NativeCommandBinding>,
        processes: &'a [NativeProcess],
    ) -> Option<&'a NativeProcess> {
        binding.and_then(
            |binding: &NativeCommandBinding| -> Option<&'a NativeProcess> {
                Self::current_process(binding, processes)
            },
        )
    }

    fn current_process<'a>(
        binding: &NativeCommandBinding,
        processes: &'a [NativeProcess],
    ) -> Option<&'a NativeProcess> {
        let unique: bool = Self::pid_count(binding.identity.pid, processes) == Self::UNIQUE_PROCESS;
        let matched: Option<&NativeProcess> = Self::matched_process(binding, processes);
        Self::unique_process(unique, matched)
    }

    fn pid_count(pid: i32, processes: &[NativeProcess]) -> usize {
        processes
            .iter()
            .filter(|process: &&NativeProcess| -> bool { process.pid == pid })
            .count()
    }

    fn matched_process<'a>(
        binding: &NativeCommandBinding,
        processes: &'a [NativeProcess],
    ) -> Option<&'a NativeProcess> {
        processes
            .iter()
            .find(|process: &&NativeProcess| -> bool { Self::same_process(binding, process) })
    }

    fn unique_process(unique: bool, process: Option<&NativeProcess>) -> Option<&NativeProcess> {
        if !unique {
            return None;
        }
        process
    }

    fn measured(
        &mut self,
        binding: &NativeCommandBinding,
        process: &NativeProcess,
        time: (f64, f64),
    ) -> NativeCommandObservation {
        let mut state: NativeCommandSample = self.state(binding);
        let duration: Option<f64> = state
            .monotonic
            .map(|previous: f64| -> f64 { time.1 - previous });
        let metrics: NativeProcessMetrics =
            self.counter
                .measure(process, state.process.as_ref(), (None, None), duration);
        Self::totals(&mut state.totals, &metrics);
        Self::history(&mut state.history, Some(&metrics), time.0);
        state.process = Some(process.clone());
        state.monotonic = Some(time.1);
        let observation = Self::observation(binding, Some(metrics), &state, time.0);
        self.states.insert(binding.id.clone(), state);
        observation
    }

    fn state(&self, binding: &NativeCommandBinding) -> NativeCommandSample {
        self.states
            .get(&binding.id)
            .filter(|state: &&NativeCommandSample| -> bool {
                state.generation == binding.generation
            })
            .cloned()
            .unwrap_or_else(|| -> NativeCommandSample {
                NativeCommandSample {
                    generation: binding.generation,
                    ..Default::default()
                }
            })
    }

    fn totals(totals: &mut NativeResourceTotals, metrics: &NativeProcessMetrics) -> () {
        totals.cpu_seconds += metrics.cpu_seconds_delta.unwrap_or_default();
        totals.disk_read_bytes = totals
            .disk_read_bytes
            .saturating_add(metrics.disk_read_bytes_delta.unwrap_or_default());
        totals.disk_written_bytes = totals
            .disk_written_bytes
            .saturating_add(metrics.disk_written_bytes_delta.unwrap_or_default());
        totals.resident_bytes = metrics.resident_bytes.unwrap_or_default();
        totals.peak_resident_bytes = totals.peak_resident_bytes.max(totals.resident_bytes);
        totals.samples = totals.samples.saturating_add(1);
    }

    fn history(
        history: &mut Vec<Vec<Option<f64>>>,
        metrics: Option<&NativeProcessMetrics>,
        timestamp: f64,
    ) -> () {
        history.retain(|sample: &Vec<Option<f64>>| -> bool {
            [
                sample[0].unwrap_or_default() >= timestamp - Self::HISTORY_SECONDS,
                sample[0].unwrap_or_default() <= timestamp,
            ]
            .into_iter()
            .all(std::convert::identity)
        });
        history.push(vec![
            Some(timestamp),
            metrics
                .and_then(|metrics: &NativeProcessMetrics| -> Option<f64> { metrics.cpu_percent }),
            metrics.and_then(|metrics: &NativeProcessMetrics| -> Option<f64> {
                metrics
                    .resident_bytes
                    .map(|bytes: i64| -> f64 { bytes as f64 })
            }),
            None,
            None,
            metrics.and_then(|metrics: &NativeProcessMetrics| -> Option<f64> {
                metrics.disk_read_per_second
            }),
            metrics.and_then(|metrics: &NativeProcessMetrics| -> Option<f64> {
                metrics.disk_write_per_second
            }),
        ]);
        let remove: usize = history.len().saturating_sub(Self::HISTORY_SAMPLES);
        history.drain(..remove);
    }

    fn observation(
        binding: &NativeCommandBinding,
        metrics: Option<NativeProcessMetrics>,
        state: &NativeCommandSample,
        timestamp: f64,
    ) -> NativeCommandObservation {
        let coverage: &str = Self::coverage(metrics.is_some());
        NativeCommandObservation {
            binding_id: binding.id.clone(),
            generation: binding.generation,
            owner: binding.owner.clone(),
            pid: binding.identity.pid,
            started: binding.identity.started.clone(),
            sampled_at: timestamp,
            scope: binding.scope.clone(),
            provenance: binding.provenance.clone(),
            validation: binding.validation.clone(),
            coverage: coverage.to_owned(),
            metrics,
            descendants: Vec::new(),
            totals: state.totals.clone(),
            history: state.history.clone(),
        }
    }

    fn coverage(measured: bool) -> &'static str {
        if measured {
            return Self::AVAILABLE;
        }
        Self::GAP
    }

    fn revoke_missing(&self, binding: Option<&NativeCommandBinding>) -> () {
        if let Some(binding) = binding {
            let _result = self.revoke_current(binding);
        }
    }

    fn revoke_current(&self, binding: &NativeCommandBinding) -> Result<(), String> {
        let registry = self
            .registry
            .as_ref()
            .ok_or_else(|| -> String { Self::STATE_ERROR.to_owned() })?;
        registry
            .lock()
            .map_err(
                |_: std::sync::PoisonError<
                    std::sync::MutexGuard<'_, NativeCommandBindingRegistryService>,
                >|
                 -> String { Self::STATE_ERROR.to_owned() },
            )?
            .revoke(&binding.id, binding.generation);
        Ok(())
    }

    fn missing_bindings(
        &mut self,
        captured: &[NativeCommandBinding],
        current: &[NativeCommandBinding],
        time: (f64, f64),
    ) -> Vec<NativeCommandObservation> {
        let mut result: Vec<NativeCommandObservation> = Vec::new();
        for binding in captured {
            let selected: Option<&NativeCommandBinding> = Self::current(binding, current);
            self.append_missing(selected, time, &mut result);
        }
        result
    }

    fn append_missing(
        &mut self,
        binding: Option<&NativeCommandBinding>,
        time: (f64, f64),
        result: &mut Vec<NativeCommandObservation>,
    ) -> () {
        if let Some(binding) = binding {
            let mut state: NativeCommandSample = self.state(binding);
            state.process = None;
            state.monotonic = None;
            Self::history(&mut state.history, None, time.0);
            result.push(Self::observation(binding, None, &state, time.0));
            self.states.insert(binding.id.clone(), state);
        }
    }

    fn retain_current(&mut self, time: (f64, f64)) -> () {
        let current: Vec<NativeCommandBinding> = self.active(time).unwrap_or_default();
        let identifiers: BTreeSet<String> = current
            .iter()
            .map(|binding: &NativeCommandBinding| -> String { binding.id.clone() })
            .collect();
        self.states
            .retain(|identifier: &String, _: &mut NativeCommandSample| -> bool {
                identifiers.contains(identifier)
            });
        self.latest
            .retain(|observation: &NativeCommandObservation| -> bool {
                identifiers.contains(&observation.binding_id)
            });
    }
}
