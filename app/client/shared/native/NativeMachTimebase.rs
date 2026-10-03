#[repr(C)]
#[derive(Default)]
pub struct NativeMachTimebase {
    pub numerator: u32,
    pub denominator: u32,
}
