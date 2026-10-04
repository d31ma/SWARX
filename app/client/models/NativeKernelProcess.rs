// The identity and CPU fields of one Linux /proc/<pid>/stat line, in kernel clock ticks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeKernelProcess {
    pub pid: i32,
    pub parent: i32,
    pub cpu_ticks: u64,
    pub start_ticks: u64,
}
