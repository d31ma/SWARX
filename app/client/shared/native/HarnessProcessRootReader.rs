use crate::models::{NativeCommandIdentity, NativeProcess};
use crate::utils::NativeCommandContextUtils;
use crate::ProcessIdentityReader;

/// Finds the hook sender's nearest ancestor that is the exact per-session harness executable.
/// Only PID, birth, user and executable path are read; never arguments, environment or cwd.
pub struct HarnessProcessRootReader {
    reader: Box<dyn ProcessIdentityReader>,
    user: i64,
    origin: i32,
}

impl HarnessProcessRootReader {
    const MAXIMUM_DEPTH: usize = 8;
    const LAUNCH_PROCESS: i32 = 1;

    pub fn new(reader: Box<dyn ProcessIdentityReader>, user: i64, origin: i32) -> Self {
        Self {
            reader,
            user,
            origin,
        }
    }

    pub fn root(&mut self, family: &str) -> Option<NativeCommandIdentity> {
        let user: i64 = self.user;
        let reader: &mut Box<dyn ProcessIdentityReader> = &mut self.reader;
        let mut next: i32 = self.origin;
        (0..Self::MAXIMUM_DEPTH)
            .map_while(|_: usize| -> Option<NativeProcess> {
                let process: NativeProcess = Self::owned(reader, user, next)?;
                next = process.parent;
                Some(process)
            })
            .find(|process: &NativeProcess| -> bool {
                NativeCommandContextUtils::session_root(family, &process.executable)
            })
            .map(Self::identity)
    }

    fn owned(
        reader: &mut Box<dyn ProcessIdentityReader>,
        user: i64,
        pid: i32,
    ) -> Option<NativeProcess> {
        let readable: bool = pid > Self::LAUNCH_PROCESS;
        readable
            .then(|| -> Option<NativeProcess> { reader.read_identity(pid).ok().flatten() })
            .flatten()
            .filter(|process: &NativeProcess| -> bool {
                [process.pid == pid, process.user == user]
                    .into_iter()
                    .all(std::convert::identity)
            })
    }

    fn identity(process: NativeProcess) -> NativeCommandIdentity {
        NativeCommandIdentity {
            pid: process.pid,
            started: process.started,
            user: process.user,
            executable: process.executable,
        }
    }
}
