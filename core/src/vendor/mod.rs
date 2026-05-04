pub mod protocol;

use crate::config::ConfigManager;
use crate::hid::types::CustomReport;
use crate::power::PowerManager;
use crate::route::HidRouter;
use crate::runtime::EVENT_QUEUE;
use crate::utils::critical::interrupt_free;
use core::cell::UnsafeCell;
use core::ptr;
use protocol::{
    CUSTOM_STATUS_BAD_LENGTH, CUSTOM_STATUS_INTERNAL_ERROR, CUSTOM_STATUS_INVALID_COMMAND,
    ProtocolStats, encode_error_response, handle_request, parse_frame, preferred_response_route,
};

pub const VENDOR_RX_PAYLOAD_CAPACITY: usize = 64;
const VENDOR_RX_QUEUE_CAPACITY: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VendorRxReport {
    pub route: u8,
    pub len: u16,
    pub timestamp: u32,
    pub data: [u8; VENDOR_RX_PAYLOAD_CAPACITY],
}

impl VendorRxReport {
    fn new(route: u8, len: u16, timestamp: u32, data: [u8; VENDOR_RX_PAYLOAD_CAPACITY]) -> Self {
        Self {
            route,
            len,
            timestamp,
            data,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VendorRxStats {
    pub dropped: u16,
    pub too_large: u16,
}

struct VendorRxQueueInner {
    buf: [Option<VendorRxReport>; VENDOR_RX_QUEUE_CAPACITY],
    head: usize,
    tail: usize,
    len: usize,
    dropped: u16,
    too_large: u16,
}

impl VendorRxQueueInner {
    const fn new() -> Self {
        Self {
            buf: [None; VENDOR_RX_QUEUE_CAPACITY],
            head: 0,
            tail: 0,
            len: 0,
            dropped: 0,
            too_large: 0,
        }
    }

    fn push(&mut self, report: VendorRxReport) -> bool {
        if self.len == VENDOR_RX_QUEUE_CAPACITY {
            self.dropped = self.dropped.saturating_add(1);
            return false;
        }

        self.buf[self.tail] = Some(report);
        self.tail = (self.tail + 1) % VENDOR_RX_QUEUE_CAPACITY;
        self.len += 1;
        true
    }

    fn pop(&mut self) -> Option<VendorRxReport> {
        if self.len == 0 {
            return None;
        }

        let report = self.buf[self.head].take();
        self.head = (self.head + 1) % VENDOR_RX_QUEUE_CAPACITY;
        self.len -= 1;
        report
    }

    fn mark_too_large(&mut self) {
        self.too_large = self.too_large.saturating_add(1);
    }

    fn stats(&self) -> VendorRxStats {
        VendorRxStats {
            dropped: self.dropped,
            too_large: self.too_large,
        }
    }
}

pub struct VendorRxQueue {
    inner: UnsafeCell<VendorRxQueueInner>,
}

// SAFETY: 所有访问由 CH585 单核临界区串行化
unsafe impl Sync for VendorRxQueue {}

impl VendorRxQueue {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(VendorRxQueueInner::new()),
        }
    }

    pub unsafe fn copy_from_ptr(
        &self,
        route: u8,
        ptr: *const u8,
        len: u16,
        timestamp: u32,
    ) -> bool {
        let len_usize = len as usize;
        if ptr.is_null() || len_usize > VENDOR_RX_PAYLOAD_CAPACITY {
            interrupt_free(|| unsafe { (&mut *self.inner.get()).mark_too_large() });
            return false;
        }

        let mut data = [0u8; VENDOR_RX_PAYLOAD_CAPACITY];
        if len_usize != 0 {
            // SAFETY: 调用方保证 callback 期间 `ptr..ptr+len` 有效
            unsafe { ptr::copy_nonoverlapping(ptr, data.as_mut_ptr(), len_usize) };
        }

        let report = VendorRxReport::new(route, len, timestamp, data);
        interrupt_free(|| unsafe { (&mut *self.inner.get()).push(report) })
    }

    pub fn pop(&self) -> Option<VendorRxReport> {
        interrupt_free(|| unsafe { (&mut *self.inner.get()).pop() })
    }

    pub fn stats(&self) -> VendorRxStats {
        interrupt_free(|| unsafe { (&*self.inner.get()).stats() })
    }
}

pub static VENDOR_RX_QUEUE: VendorRxQueue = VendorRxQueue::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingVendorTx {
    pub route: u8,
    pub report: CustomReport,
}

pub struct VendorRuntime {
    pending_rx: bool,
    pending_tx: Option<PendingVendorTx>,
    last_route: u8,
    last_len: u16,
    last_timestamp: u32,
    processed_count: u16,
    stats: ProtocolStats,
}

impl VendorRuntime {
    pub fn new() -> Self {
        Self {
            pending_rx: false,
            pending_tx: None,
            last_route: 0,
            last_len: 0,
            last_timestamp: 0,
            processed_count: 0,
            stats: ProtocolStats::default(),
        }
    }

    pub fn mark_rx_pending(&mut self) {
        self.pending_rx = true;
    }

    pub fn has_pending_tx(&self) -> bool {
        self.pending_tx.is_some()
    }

    pub fn take_pending_tx(&mut self) -> Option<PendingVendorTx> {
        self.pending_tx.take()
    }

    pub fn requeue_pending_tx(&mut self, tx: PendingVendorTx) {
        self.pending_tx = Some(tx);
    }

    pub fn stats(&self) -> ProtocolStats {
        self.stats
    }

    pub fn poll(&mut self, router: &HidRouter, config: &mut ConfigManager, power: &PowerManager) {
        while let Some(report) = VENDOR_RX_QUEUE.pop() {
            self.last_route = report.route;
            self.last_len = report.len;
            self.last_timestamp = report.timestamp;
            self.processed_count = self.processed_count.saturating_add(1);
            self.handle_rx_report(report, router, config, power);
        }

        self.pending_rx = false;
    }

    fn handle_rx_report(
        &mut self,
        report: VendorRxReport,
        router: &HidRouter,
        config: &mut ConfigManager,
        power: &PowerManager,
    ) {
        let len = report.len as usize;
        let buf = &report.data[..len.min(VENDOR_RX_PAYLOAD_CAPACITY)];
        let route = preferred_response_route(router, report.route);

        let event_queue_stats = EVENT_QUEUE.stats();
        let vendor_rx_stats = VENDOR_RX_QUEUE.stats();

        let mut response = CustomReport::default();
        match parse_frame(buf) {
            Ok(frame) => match handle_request(
                frame,
                router,
                config,
                power,
                self.stats,
                vendor_rx_stats,
                event_queue_stats,
                &mut response,
            ) {
                Ok(()) => {
                    self.stats.rx_ok = self.stats.rx_ok.saturating_add(1);
                    self.queue_response(route.as_ffi(), response);
                }
                Err(status) => {
                    if status == CUSTOM_STATUS_INVALID_COMMAND {
                        self.stats.rx_unsupported = self.stats.rx_unsupported.saturating_add(1);
                    } else {
                        self.stats.rx_invalid = self.stats.rx_invalid.saturating_add(1);
                    }

                    if status == CUSTOM_STATUS_INTERNAL_ERROR {
                        let _ = encode_error_response(
                            frame.command,
                            frame.sequence,
                            status,
                            &mut response,
                        );
                    }
                    if response.len != 0 {
                        self.queue_response(route.as_ffi(), response);
                    }
                }
            },
            Err(_) => {
                self.stats.rx_invalid = self.stats.rx_invalid.saturating_add(1);
                if encode_error_response(0, 0, CUSTOM_STATUS_BAD_LENGTH, &mut response).is_ok() {
                    self.queue_response(route.as_ffi(), response);
                }
            }
        }
    }

    fn queue_response(&mut self, route: u8, report: CustomReport) {
        if route == 0 {
            self.stats.tx_dropped_no_route = self.stats.tx_dropped_no_route.saturating_add(1);
            return;
        }

        self.pending_tx = Some(PendingVendorTx { route, report });
        self.stats.tx_generated = self.stats.tx_generated.saturating_add(1);
    }
}

impl Default for VendorRuntime {
    fn default() -> Self {
        Self::new()
    }
}
