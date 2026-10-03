use crate::models::NativeSnapshot;

#[Service]
#[Lifetime({"value":"application"})]
pub struct ApplicationCollectorService {
    monitor: Box<MonitorService>,
}

impl Default for ApplicationCollectorService {
    fn default() -> Self {
        let monitor: MonitorService = ApplicationCompositionService::monitor();
        Self {
            monitor: Box::new(monitor),
        }
    }
}

impl ApplicationCollectorService {
    pub fn read(&self) -> NativeSnapshot {
        self.monitor.read()
    }

    pub fn observation_time(&self) -> f64 {
        self.monitor.observation_time()
    }

    #[OnStop]
    fn shutdown(&self) -> () {
        self.monitor.stop();
    }
}
