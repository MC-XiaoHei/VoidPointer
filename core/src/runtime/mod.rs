use crate::config::ConfigManager;
use crate::ffi::bindings::{c_vp_request_core_poll, c_vp_rtc_millis};
use crate::power::PowerManager;
use crate::route::HidRouter;
use crate::utils::global::MainLoopGlobal;
use crate::vendor::VendorRuntime;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use log::debug;

pub static RUNTIME: MainLoopGlobal<Runtime> = MainLoopGlobal::new();

pub static POLL_RUNNING: AtomicBool = AtomicBool::new(false);
pub static POLL_PENDING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DirtyFlags {
    pub input: bool,
    pub motion: bool,
    pub report: bool,
    pub power: bool,
    pub config: bool,
}

impl DirtyFlags {
    pub fn any(self) -> bool {
        self.input || self.motion || self.report || self.power || self.config
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingFlags {
    pub events: bool,
    pub hid_retry: bool,
    pub imu_fifo_read: bool,
    pub vendor_rx: bool,
    pub config_save: bool,
    pub power_eval: bool,
}

impl PendingFlags {
    pub fn any(self) -> bool {
        self.events
            || self.hid_retry
            || self.imu_fifo_read
            || self.vendor_rx
            || self.config_save
            || self.power_eval
    }
}

pub struct Runtime {
    pub router: HidRouter,
    pub power: PowerManager,
    pub config: ConfigManager,
    pub vendor: VendorRuntime,
    pub dirty: DirtyFlags,
    pub pending: PendingFlags,
    pub last_activity_ms: AtomicU32,
}

impl Runtime {
    pub fn new() -> Self {
        let now = unsafe { c_vp_rtc_millis() };
        Self {
            router: HidRouter::new(),
            power: PowerManager::new(),
            config: ConfigManager::new(),
            vendor: VendorRuntime::new(),
            dirty: DirtyFlags::default(),
            pending: PendingFlags {
                power_eval: true,
                ..PendingFlags::default()
            },
            last_activity_ms: AtomicU32::new(now),
        }
    }

    pub fn request_poll() {
        POLL_PENDING.store(true, Ordering::Release);
        unsafe { c_vp_request_core_poll() };
    }

    pub fn mark_activity(&mut self, timestamp_ms: u32) {
        self.last_activity_ms.store(timestamp_ms, Ordering::Release);
        self.dirty.power = true;
        self.pending.power_eval = true;
    }

    pub fn poll(&mut self) {
        const MAX_PASSES: usize = 4;
        let mut passes = 0;

        while passes < MAX_PASSES {
            passes += 1;
            POLL_PENDING.store(false, Ordering::Release);

            self.process_once();

            if !POLL_PENDING.load(Ordering::Acquire) && !self.pending.any() && !self.dirty.any() {
                break;
            }
        }

        if POLL_PENDING.load(Ordering::Acquire) || self.pending.any() || self.dirty.any() {
            Self::request_poll();
        }
    }

    fn process_once(&mut self) {
        if self.pending.events {
            debug!("event queue placeholder drained");
            self.pending.events = false;
        }

        if self.pending.vendor_rx {
            self.vendor.poll();
            self.pending.vendor_rx = false;
        }

        if self.pending.config_save || self.dirty.config {
            self.config.poll();
            self.pending.config_save = false;
            self.dirty.config = self.config.is_dirty();
        }

        if self.pending.power_eval || self.dirty.power {
            let now = unsafe { c_vp_rtc_millis() };
            let last_activity = self.last_activity_ms.load(Ordering::Acquire);
            self.power
                .poll(now, last_activity, self.config.is_dirty(), &self.router);
            self.pending.power_eval = false;
            self.dirty.power = false;
        }

        self.dirty.input = false;
        self.dirty.motion = false;
        self.dirty.report = false;
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
