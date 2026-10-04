use crate::NetworkReader;
use std::collections::BTreeMap;

// Linux has no unprivileged per-process network byte counter (nettop's role on macOS); per-socket
// accounting needs eBPF, netfilter or other privileged setup that Swarx must not install. The
// reader therefore always reports unavailable, the same way the macOS reader reports a failed sample.
#[derive(Default)]
#[Service]
pub struct KernelNetworkReaderService;

#[cfg(target_os = "linux")]
impl KernelNetworkReaderService {
    pub fn new() -> Self {
        Self
    }

    fn read(&self) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        Err(
            "Network counters unavailable: Linux exposes no unprivileged per-process byte counters."
                .to_owned(),
        )
    }
}

#[cfg(target_os = "linux")]
impl NetworkReader for KernelNetworkReaderService {
    fn read(&mut self) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        KernelNetworkReaderService::read(self)
    }
}
