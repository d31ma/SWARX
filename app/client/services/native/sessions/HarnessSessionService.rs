use crate::models::NativeHarnessSession;
use crate::{HarnessSessionSource, ObservationClock};
use std::cmp::Ordering as ComparisonOrder;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

// Discovery reads harness folders and databases, and any of those reads can block without limit
// (a cloud-synced folder, a pending macOS privacy prompt). It therefore runs only on this
// service's own refresh thread; page reads take the latest finished result and never touch the
// filesystem. Before the first discovery finishes the result is empty with no capture time.
#[Service]
pub struct HarnessSessionService {
    sources: Arc<Vec<Box<dyn HarnessSessionSource>>>,
    latest: Arc<Mutex<(Vec<NativeHarnessSession>, Option<f64>)>>,
    stopping: Arc<AtomicBool>,
    refresh: Mutex<Option<thread::JoinHandle<()>>>,
}

impl HarnessSessionService {
    const STOP_WAIT: Duration = Duration::from_millis(500);
    const STOP_POLL: Duration = Duration::from_millis(10);

    pub fn new(sources: Vec<Box<dyn HarnessSessionSource>>) -> Self {
        Self {
            sources: Arc::new(sources),
            latest: Arc::new(Mutex::new((Vec::new(), None))),
            stopping: Arc::new(AtomicBool::new(false)),
            refresh: Mutex::new(None),
        }
    }

    // The latest finished discovery and its capture time. Never blocks on a source.
    pub fn latest(&self) -> (Vec<NativeHarnessSession>, Option<f64>) {
        Self::guard(&self.latest).clone()
    }

    // Reads every source on the caller's thread; only the refresh thread and tests call this.
    pub fn discover(&self) -> Vec<NativeHarnessSession> {
        Self::merged(&self.sources)
    }

    // Starts the single refresh thread: one discovery at a time, then a pause of `interval`.
    // A discovery that hangs holds only this thread, so no second one starts beside it.
    pub fn start(&self, clock: Arc<dyn ObservationClock>, interval: Duration) -> () {
        let mut refresh: MutexGuard<'_, Option<thread::JoinHandle<()>>> =
            Self::guard(&self.refresh);
        if refresh.is_some() || self.stopping.load(Ordering::Acquire) {
            return;
        }
        let sources: Arc<Vec<Box<dyn HarnessSessionSource>>> = Arc::clone(&self.sources);
        let latest: Arc<Mutex<(Vec<NativeHarnessSession>, Option<f64>)>> = Arc::clone(&self.latest);
        let stopping: Arc<AtomicBool> = Arc::clone(&self.stopping);
        *refresh = Some(thread::spawn(move || -> () {
            Self::refresh_loop(&sources, &latest, &stopping, clock.as_ref(), interval)
        }));
    }

    // A discovery blocked in the kernel cannot be interrupted, so stopping waits at most
    // STOP_WAIT and then detaches the thread: it ends after its blocked call returns, or with
    // the process on Quit. Its result is never read again.
    pub fn stop(&self) -> () {
        self.stopping.store(true, Ordering::Release);
        let handle: Option<thread::JoinHandle<()>> = Self::guard(&self.refresh).take();
        handle.map(Self::finish);
    }

    fn refresh_loop(
        sources: &[Box<dyn HarnessSessionSource>],
        latest: &Mutex<(Vec<NativeHarnessSession>, Option<f64>)>,
        stopping: &AtomicBool,
        clock: &dyn ObservationClock,
        interval: Duration,
    ) -> () {
        while !stopping.load(Ordering::Acquire) {
            let sessions: Vec<NativeHarnessSession> = Self::merged(sources);
            *Self::guard(latest) = (sessions, Some(clock.now().0));
            thread::park_timeout(interval);
        }
    }

    fn finish(handle: thread::JoinHandle<()>) -> () {
        handle.thread().unpark();
        let deadline: Instant = Instant::now() + Self::STOP_WAIT;
        while !handle.is_finished() && Instant::now() < deadline {
            thread::sleep(Self::STOP_POLL);
        }
        if handle.is_finished() {
            let _ = handle.join();
        }
    }

    fn merged(sources: &[Box<dyn HarnessSessionSource>]) -> Vec<NativeHarnessSession> {
        let mut sessions: Vec<NativeHarnessSession> = Self::collect(sources);
        sessions.sort_by(Self::order);
        sessions.dedup_by(Self::same_session);
        sessions
    }

    fn collect(sources: &[Box<dyn HarnessSessionSource>]) -> Vec<NativeHarnessSession> {
        let mut sessions: Vec<NativeHarnessSession> = Vec::new();
        for source in sources {
            sessions.extend(source.sessions());
        }
        sessions
    }

    fn guard<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
        value.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn order(left: &NativeHarnessSession, right: &NativeHarnessSession) -> ComparisonOrder {
        left.family
            .cmp(&right.family)
            .then_with(|| -> ComparisonOrder { left.id.cmp(&right.id) })
            .then_with(|| -> ComparisonOrder {
                Self::activity(right).total_cmp(&Self::activity(left))
            })
            .then_with(|| -> ComparisonOrder { left.app_id.cmp(&right.app_id) })
            .then_with(|| -> ComparisonOrder { left.title.cmp(&right.title) })
    }

    fn activity(session: &NativeHarnessSession) -> f64 {
        session.last_activity_at.unwrap_or(f64::NEG_INFINITY)
    }

    fn same_session(later: &mut NativeHarnessSession, kept: &mut NativeHarnessSession) -> bool {
        [later.family == kept.family, later.id == kept.id]
            .into_iter()
            .all(std::convert::identity)
    }
}

impl Drop for HarnessSessionService {
    fn drop(&mut self) -> () {
        self.stop();
    }
}
