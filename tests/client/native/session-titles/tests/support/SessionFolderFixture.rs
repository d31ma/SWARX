use std::fs;
use std::path::PathBuf;

pub struct SessionFolderFixture {
    pub root: PathBuf,
}

impl SessionFolderFixture {
    pub const GROUP: &'static str =
        "11111111-1111-4111-8111-111111111111/22222222-2222-4222-8222-222222222222";

    pub fn new(name: &str) -> Self {
        let folder: String = format!("swarx-session-titles-{}-{name}", std::process::id());
        let root: PathBuf = std::env::temp_dir().join(folder);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path: PathBuf = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }

    pub fn record(id: &str, title: &str, last_activity_at: u64) -> String {
        serde_json::json!({
            "sessionId": format!("local_{id}"),
            "cliSessionId": id,
            "cwd": "/Users/example/Projects/SWARX",
            "title": title,
            "titleSource": "user",
            "isArchived": false,
            "lastActivityAt": last_activity_at,
        })
        .to_string()
    }
}

impl Drop for SessionFolderFixture {
    fn drop(&mut self) -> () {
        let _ = fs::remove_dir_all(&self.root);
    }
}
