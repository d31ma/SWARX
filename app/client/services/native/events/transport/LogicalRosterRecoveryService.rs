use crate::models::{NativeLogicalAgent, NativeSessionBindingRecord};
use std::collections::HashSet;

#[Service]
pub struct LogicalRosterRecoveryService {
    store: Result<LogicalRosterStoreService, String>,
    recovered: Vec<NativeLogicalAgent>,
    failure: Option<String>,
    history: LogicalExecutionHistoryService,
    bindings: Option<Vec<NativeSessionBindingRecord>>,
    binding_failure: Option<String>,
}

impl LogicalRosterRecoveryService {
    const LIMIT: usize = 100;
    const FAILURE_PREFIX: &'static str = "Roster recovery unavailable";
    const BINDING_PREFIX: &'static str = "Session link recovery unavailable";

    pub fn new(store: Result<LogicalRosterStoreService, String>) -> Self {
        let failure: Option<String> = store.as_ref().err().cloned().map(Self::failure_text);
        Self {
            store,
            recovered: Vec::new(),
            failure,
            history: LogicalExecutionHistoryService::default(),
            bindings: None,
            binding_failure: None,
        }
    }

    pub fn restore(&mut self, now: f64) -> () {
        let result: Result<Vec<NativeLogicalAgent>, String> = self.load(now);
        self.restoration(result);
    }

    pub fn persist(&mut self, live: &[NativeLogicalAgent], now: f64) -> () {
        let rows: Vec<NativeLogicalAgent> = self.rows(live, now);
        let result: Result<(), String> = self.save(&rows);
        self.failure = result.err().map(Self::failure_text);
    }

    pub fn rows(&self, live: &[NativeLogicalAgent], now: f64) -> Vec<NativeLogicalAgent> {
        let identities: HashSet<&str> = Self::identities(live);
        let available: usize = Self::LIMIT.saturating_sub(live.len());
        let mut rows: Vec<NativeLogicalAgent> = self.live_rows(live);
        rows.extend(self.cached_rows(&identities, available, now));
        rows
    }

    pub fn errors(&self) -> Vec<String> {
        let mut errors: Vec<String> = Self::error_rows(&self.failure);
        errors.extend(Self::error_rows(&self.binding_failure));
        errors
    }

    // The previous run's session links, read once at startup; persisting starts only after this,
    // so a new run never overwrites them before they are re-validated.
    pub fn restored_bindings(&mut self) -> Vec<NativeSessionBindingRecord> {
        // Without a roster store the roster failure already explains the missing recovery.
        let result: Result<Vec<NativeSessionBindingRecord>, String> = self
            .store
            .as_ref()
            .map_or(Ok(Vec::new()), LogicalRosterStoreService::load_bindings);
        self.binding_failure = result.as_ref().err().cloned().map(Self::binding_text);
        let records: Vec<NativeSessionBindingRecord> = result.unwrap_or_default();
        self.bindings = Some(records.clone());
        records
    }

    // Writes the current session links when they differ from the last saved set.
    pub fn persist_bindings(&mut self, records: Vec<NativeSessionBindingRecord>) -> () {
        let changed: bool = self
            .bindings
            .as_ref()
            .is_some_and(|saved: &Vec<NativeSessionBindingRecord>| -> bool { *saved != records });
        self.save_changed(records, changed);
    }

    fn save_changed(&mut self, records: Vec<NativeSessionBindingRecord>, changed: bool) -> () {
        if changed {
            let result: Result<(), String> = self.store.as_ref().map_or(
                Ok(()),
                |store: &LogicalRosterStoreService| -> Result<(), String> {
                    store.save_bindings(&records)
                },
            );
            // A failed write keeps the old set, so the next read retries it.
            self.binding_failure = result.as_ref().err().cloned().map(Self::binding_text);
            self.bindings = result
                .map(|_saved: ()| -> Vec<NativeSessionBindingRecord> { records })
                .ok()
                .or(self.bindings.take());
        }
    }

    fn binding_text(failure: String) -> String {
        Self::prefixed(Self::BINDING_PREFIX, failure)
    }

    fn load(&self, now: f64) -> Result<Vec<NativeLogicalAgent>, String> {
        self.store.as_ref().map_err(Clone::clone)?.load(now)
    }

    fn save(&self, rows: &[NativeLogicalAgent]) -> Result<(), String> {
        self.store.as_ref().map_err(Clone::clone)?.save(rows)
    }

    fn restoration(&mut self, result: Result<Vec<NativeLogicalAgent>, String>) -> () {
        match result {
            Ok(rows) => {
                self.recovered = rows;
                self.failure = None;
            }
            Err(failure) => {
                self.recovered.clear();
                self.failure = Some(Self::failure_text(failure));
            }
        }
    }

    fn identities(rows: &[NativeLogicalAgent]) -> HashSet<&str> {
        rows.iter()
            .map(|row: &NativeLogicalAgent| -> &str { row.id.as_str() })
            .collect()
    }

    fn live_rows(&self, live: &[NativeLogicalAgent]) -> Vec<NativeLogicalAgent> {
        live.iter()
            .map(|row: &NativeLogicalAgent| -> NativeLogicalAgent { self.live_row(row) })
            .collect()
    }

    fn live_row(&self, row: &NativeLogicalAgent) -> NativeLogicalAgent {
        let recovered: Option<&NativeLogicalAgent> = self
            .recovered
            .iter()
            .find(|previous: &&NativeLogicalAgent| -> bool { Self::same_source(previous, row) });
        let mut result: NativeLogicalAgent = row.clone();
        let previous: &[crate::models::NativeExecutionRecord] = recovered
            .map(
                |previous: &NativeLogicalAgent| -> &[crate::models::NativeExecutionRecord] {
                    previous.executions.as_slice()
                },
            )
            .unwrap_or_default();
        result.executions = self.history.retained(&row.executions, previous);
        result
    }

    fn same_source(previous: &NativeLogicalAgent, row: &NativeLogicalAgent) -> bool {
        previous.id == row.id && previous.source == row.source
    }

    fn cached_rows(
        &self,
        identities: &HashSet<&str>,
        available: usize,
        now: f64,
    ) -> Vec<NativeLogicalAgent> {
        let mut rows: Vec<NativeLogicalAgent> = Vec::new();
        for row in &self.recovered {
            let aged: NativeLogicalAgent = Self::aged(row, now);
            Self::append_cached(&mut rows, aged, identities, available);
        }
        rows
    }

    fn append_cached(
        rows: &mut Vec<NativeLogicalAgent>,
        row: NativeLogicalAgent,
        identities: &HashSet<&str>,
        available: usize,
    ) -> () {
        let eligible: bool = Self::eligible(&row, identities, rows.len(), available);
        Self::append_eligible(rows, row, eligible);
    }

    fn eligible(
        row: &NativeLogicalAgent,
        identities: &HashSet<&str>,
        count: usize,
        available: usize,
    ) -> bool {
        !identities.contains(row.id.as_str()) && count < available
    }

    fn append_eligible(
        rows: &mut Vec<NativeLogicalAgent>,
        row: NativeLogicalAgent,
        eligible: bool,
    ) -> () {
        if eligible {
            rows.push(row);
        }
    }

    fn aged(row: &NativeLogicalAgent, now: f64) -> NativeLogicalAgent {
        let mut result: NativeLogicalAgent = row.clone();
        result.age_seconds = (now - result.captured_at).max(0.0);
        result
    }

    fn error_rows(failure: &Option<String>) -> Vec<String> {
        failure.iter().cloned().collect()
    }

    fn failure_text(failure: String) -> String {
        Self::prefixed(Self::FAILURE_PREFIX, failure)
    }

    fn prefixed(prefix: &str, failure: String) -> String {
        format!("{prefix}: {failure}")
    }
}
