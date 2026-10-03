use crate::models::NativeExecutionRecord;
use std::collections::HashSet;

#[derive(Default)]
#[Service]
pub struct LogicalExecutionHistoryService {}

impl LogicalExecutionHistoryService {
    const LIMIT: usize = 20;

    pub fn retained(
        &self,
        live: &[NativeExecutionRecord],
        recovered: &[NativeExecutionRecord],
    ) -> Vec<NativeExecutionRecord> {
        let identities: HashSet<&str> = live
            .iter()
            .map(|item: &NativeExecutionRecord| -> &str { item.id.as_str() })
            .collect();
        let available: usize = Self::LIMIT.saturating_sub(live.len());
        let mut records: Vec<NativeExecutionRecord> =
            Self::cached(recovered, &identities, available);
        records.extend_from_slice(live);
        records.sort_by(
            |left: &NativeExecutionRecord, right: &NativeExecutionRecord| -> std::cmp::Ordering {
                left.started_at.total_cmp(&right.started_at)
            },
        );
        records
    }

    fn cached(
        recovered: &[NativeExecutionRecord],
        identities: &HashSet<&str>,
        available: usize,
    ) -> Vec<NativeExecutionRecord> {
        let mut records: Vec<NativeExecutionRecord> = Vec::new();
        for item in recovered.iter().rev() {
            Self::append(&mut records, item, identities, available);
        }
        records.reverse();
        records
    }

    fn append(
        records: &mut Vec<NativeExecutionRecord>,
        item: &NativeExecutionRecord,
        identities: &HashSet<&str>,
        available: usize,
    ) -> () {
        let eligible: bool = Self::eligible(item, identities, records.len(), available);
        Self::append_eligible(records, item, eligible);
    }

    fn eligible(
        item: &NativeExecutionRecord,
        identities: &HashSet<&str>,
        count: usize,
        available: usize,
    ) -> bool {
        !identities.contains(item.id.as_str()) && count < available
    }

    fn append_eligible(
        records: &mut Vec<NativeExecutionRecord>,
        item: &NativeExecutionRecord,
        eligible: bool,
    ) -> () {
        if eligible {
            records.push(item.clone());
        }
    }
}
