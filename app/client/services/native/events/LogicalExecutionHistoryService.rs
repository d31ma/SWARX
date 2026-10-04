use crate::models::NativeExecutionRecord;
use std::collections::HashSet;

#[derive(Default)]
#[Service]
pub struct LogicalExecutionHistoryService {}

impl LogicalExecutionHistoryService {
    const LIMIT: usize = 20;
    const RUNNING: &'static str = "running";

    /// Merges live records with the ones recovered after a restart, keeping at most LIMIT.
    /// Live running actions come first, then background launches (live before recovered), so a
    /// long-lived background task keeps its reported launch title however much foreground
    /// history follows it; the newest remaining live, then recovered, history fills the rest.
    pub fn retained(
        &self,
        live: &[NativeExecutionRecord],
        recovered: &[NativeExecutionRecord],
    ) -> Vec<NativeExecutionRecord> {
        let identities: HashSet<&str> = live
            .iter()
            .map(|item: &NativeExecutionRecord| -> &str { item.id.as_str() })
            .collect();
        let mut candidates: Vec<(bool, &NativeExecutionRecord)> = live
            .iter()
            .map(|item: &NativeExecutionRecord| -> (bool, &NativeExecutionRecord) { (true, item) })
            .chain(
                recovered
                    .iter()
                    .filter(|item: &&NativeExecutionRecord| -> bool {
                        !identities.contains(item.id.as_str())
                    })
                    .map(
                        |item: &NativeExecutionRecord| -> (bool, &NativeExecutionRecord) {
                            (false, item)
                        },
                    ),
            )
            .collect();
        candidates.sort_by(
            |left: &(bool, &NativeExecutionRecord),
             right: &(bool, &NativeExecutionRecord)|
             -> std::cmp::Ordering {
                Self::rank(left)
                    .cmp(&Self::rank(right))
                    .then(right.1.started_at.total_cmp(&left.1.started_at))
            },
        );
        let mut records: Vec<NativeExecutionRecord> = candidates
            .into_iter()
            .take(Self::LIMIT)
            .map(
                |(_live, item): (bool, &NativeExecutionRecord)| -> NativeExecutionRecord {
                    item.clone()
                },
            )
            .collect();
        records.sort_by(
            |left: &NativeExecutionRecord, right: &NativeExecutionRecord| -> std::cmp::Ordering {
                left.started_at.total_cmp(&right.started_at)
            },
        );
        records
    }

    /// Lower ranks are kept first: (not a live running action, not a background launch, not live).
    fn rank(candidate: &(bool, &NativeExecutionRecord)) -> (bool, bool, bool) {
        let (live, item): (bool, &NativeExecutionRecord) = *candidate;
        let running: bool = live && item.status == Self::RUNNING;
        let launch: bool = item.background == Some(true);
        (!running, !launch, !live)
    }
}
