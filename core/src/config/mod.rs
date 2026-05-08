pub const CURRENT_CONFIG_VERSION: u16 = 1;

pub struct ConfigManager {
    dirty: bool,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self { dirty: false }
    }

    pub fn current_config_version(&self) -> u16 {
        CURRENT_CONFIG_VERSION
    }

    pub fn current_payload_len(&self) -> u32 {
        0
    }

    pub fn current_payload_crc32(&self) -> u32 {
        0
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn poll(&mut self) {
        // TODO: DataFlash 持久化路径接入后，在保存成功时清 dirty
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
