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
        // DataFlash-backed config save/load is intentionally deferred to a later task.
        // Keep dirty set until the real save path can clear it.
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
