use observatory_measurement_tests::ProcessIdentityReader;
use observatory_measurement_tests::models::NativeProcess;

pub struct CommandMeasurementIdentityReader {
    process: NativeProcess,
}

impl CommandMeasurementIdentityReader {
    pub fn new(process: NativeProcess) -> Self {
        Self { process }
    }
}

impl ProcessIdentityReader for CommandMeasurementIdentityReader {
    fn read_identity(&mut self, pid: i32) -> Result<Option<NativeProcess>, String> {
        Ok(Some(self.process.clone())
            .filter(|process: &NativeProcess| -> bool { process.pid == pid }))
    }
}
