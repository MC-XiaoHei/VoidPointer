pub struct VendorRuntime {
    pending_rx: bool,
}

impl VendorRuntime {
    pub fn new() -> Self {
        Self { pending_rx: false }
    }

    pub fn mark_rx_pending(&mut self) {
        self.pending_rx = true;
    }

    pub fn poll(&mut self) {
        // Vendor/WebHID frame parsing is deferred to a later implementation step.
        self.pending_rx = false;
    }
}

impl Default for VendorRuntime {
    fn default() -> Self {
        Self::new()
    }
}
