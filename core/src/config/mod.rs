pub struct ConfigManager {
    dirty: bool,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self { dirty: false }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn poll(&mut self) {
        // 等真实 DataFlash 保存路径接入后再清 dirty
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
