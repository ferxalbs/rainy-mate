use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct AgentKillSwitch {
    triggered: Arc<AtomicBool>,
}

impl AgentKillSwitch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trigger(&self) {
        self.triggered.store(true, Ordering::Relaxed);
    }

    pub fn clear(&self) {
        self.triggered.store(false, Ordering::Relaxed);
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::Relaxed)
    }
}
