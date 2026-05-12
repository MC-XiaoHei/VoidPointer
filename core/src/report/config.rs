use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReportConfig {
    pub report_hz: f32,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self { report_hz: 1000.0 }
    }
}
