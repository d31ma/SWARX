use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct NativeSessionProcessClaims {
    pub roots: BTreeMap<(i32, String), BTreeSet<String>>,
    pub descendants: BTreeMap<(i32, String), BTreeSet<String>>,
    pub births: BTreeMap<i32, BTreeSet<String>>,
}
