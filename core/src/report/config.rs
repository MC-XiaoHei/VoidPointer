#[derive(Debug, Clone, Copy)]
pub struct ReportConfig {
    pub report_hz: f32,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self { report_hz: 1000.0 }
    }
}
