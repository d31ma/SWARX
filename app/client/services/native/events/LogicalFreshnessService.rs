use crate::models::{NativeExecutionRecord, NativeLogicalAgent, NativeTaskRecord};

#[derive(Default)]
#[Service]
pub struct LogicalFreshnessService {
    policy: LogicalEventPolicyService,
}

impl LogicalFreshnessService {
    pub fn row(&self, source: &NativeLogicalAgent, now: f64) -> NativeLogicalAgent {
        let mut row: NativeLogicalAgent = source.clone();
        row.age_seconds = (now - row.captured_at).max(0.0);
        row.stale = row.age_seconds > LogicalEventPolicyService::STALE_SECONDS;
        self.mark_agent(&mut row);
        row.task = row
            .task
            .map(|task: NativeTaskRecord| -> NativeTaskRecord { self.task(task, row.stale) });
        self.mark_executions(&mut row);
        row
    }

    pub fn execution(&self, item: &mut NativeExecutionRecord, stale: bool) -> () {
        let eligible: bool = self.unfinished(&item.status, stale);
        self.mark_execution(item, eligible);
    }

    fn mark_agent(&self, row: &mut NativeLogicalAgent) -> () {
        if row.stale {
            row.reported_status = Some(row.status.clone());
            row.status = LogicalEventPolicyService::STALE.to_string();
        }
    }

    fn task(&self, mut task: NativeTaskRecord, stale: bool) -> NativeTaskRecord {
        let eligible: bool = self.unfinished(&task.status, stale);
        self.mark_task(&mut task, eligible);
        task
    }

    fn mark_task(&self, task: &mut NativeTaskRecord, eligible: bool) -> () {
        if eligible {
            task.reported_status = Some(task.status.clone());
            task.status = LogicalEventPolicyService::STALE.to_string();
        }
    }

    fn mark_executions(&self, row: &mut NativeLogicalAgent) -> () {
        for item in &mut row.executions {
            self.execution(item, row.stale);
        }
    }

    fn mark_execution(&self, item: &mut NativeExecutionRecord, eligible: bool) -> () {
        if eligible {
            item.reported_status = Some(item.status.clone());
            item.status = LogicalEventPolicyService::STALE.to_string();
        }
    }

    fn unfinished(&self, status: &str, stale: bool) -> bool {
        stale && !self.policy.is_final(status)
    }
}
