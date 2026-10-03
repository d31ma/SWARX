use crate::models::NativeProcess;

pub struct EmitterIdentityFixture;

impl EmitterIdentityFixture {
    pub fn arguments(value: &str) -> Vec<String> {
        vec!["--identity".to_owned(), value.to_owned()]
    }

    pub fn process() -> NativeProcess {
        NativeProcess {
            pid: 42,
            started: "1700000000:123456".to_owned(),
            user: 501,
            executable: "/fixture/bin/command".to_owned(),
            ..NativeProcess::default()
        }
    }
}
