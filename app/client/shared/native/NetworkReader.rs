use std::collections::BTreeMap;

pub trait NetworkReader: Send {
    fn read(&mut self) -> Result<BTreeMap<i32, (i64, i64)>, String>;
}
