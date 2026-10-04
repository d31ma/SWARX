#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NativeWindowsTimes {
    pub creation: u64,
    pub kernel: u64,
    pub user: u64,
}
