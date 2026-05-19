pub mod protocol;

use crate::config::ConfigManager;
use crate::hid::types::CustomReport;
use crate::power::PowerManager;
use crate::route::HidRouter;
use crate::sync::spsc::SpscQueue;
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

pub struct VendorRxQueue {
    inner: SpscQueue<VendorRxReport, VENDOR_RX_QUEUE_CAPACITY>,
}

impl VendorRxQueue {
    pub const fn new() -> Self {
        Self {
            inner: SpscQueue::from_array(
                [VendorRxReport {
                    route: 0,
                    len: 0,
                    timestamp: 0,
                    data: [0u8; VENDOR_RX_PAYLOAD_CAPACITY],
                }; VENDOR_RX_QUEUE_CAPACITY],
            ),
        }
    }

    /// 只能在 ISR 上下文调用
    ///
    /// # Safety
    ///
    /// - `ptr..ptr+len` 必须在调用期间有效
    pub unsafe fn copy_from_ptr(
        &self,
        route: u8,
        ptr: *const u8,
        len: u16,
        timestamp: u32,
    ) -> bool {
        let len_usize = len as usize;
        if ptr.is_null() || len_usize > VENDOR_RX_PAYLOAD_CAPACITY {
            self.inner.mark_drop_detail(1);
            return false;
        }

        let mut data = [0u8; VENDOR_RX_PAYLOAD_CAPACITY];
        if len_usize != 0 {
            unsafe { core::ptr::copy_nonoverlapping(ptr, data.as_mut_ptr(), len_usize) };
        }

        let report = VendorRxReport::new(route, len, timestamp, data);
        self.inner.push(report)
    }

    pub fn pop(&self) -> Option<VendorRxReport> {
        self.inner.pop()
    }

    pub fn stats(&self) -> VendorRxStats {
        VendorRxStats {
            dropped: self.inner.dropped() as u16,
            too_large: self.inner.drop_detail() as u16,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_queue_is_empty() {
        let q = VendorRxQueue::new();
        assert!(q.pop().is_none());
        let s = q.stats();
        assert_eq!(s.dropped, 0);
        assert_eq!(s.too_large, 0);
    }

    #[test]
    fn copy_from_ptr_valid() {
        let q = VendorRxQueue::new();
        let payload: [u8; 4] = [0xA5, 0x01, 0x01, 0x00];
        let copied = unsafe { q.copy_from_ptr(3, payload.as_ptr(), 4, 1000) };
        assert!(copied);
        let report = q.pop().unwrap();
        assert_eq!(report.route, 3);
        assert_eq!(report.len, 4);
        assert_eq!(report.timestamp, 1000);
        assert_eq!(&report.data[..4], &payload);
    }

    #[test]
    fn copy_from_ptr_null_returns_false() {
        let q = VendorRxQueue::new();
        let copied = unsafe { q.copy_from_ptr(3, core::ptr::null(), 4, 1000) };
        assert!(!copied);
        assert_eq!(q.stats().too_large, 1);
    }

    #[test]
    fn copy_from_ptr_oversized_returns_false() {
        let q = VendorRxQueue::new();
        let buf = [0u8; VENDOR_RX_PAYLOAD_CAPACITY + 1];
        let copied = unsafe { q.copy_from_ptr(3, buf.as_ptr(), buf.len() as u16, 1000) };
        assert!(!copied);
        assert_eq!(q.stats().too_large, 1);
    }

    #[test]
    fn copy_from_ptr_fifo_order() {
        let q = VendorRxQueue::new();
        let a: [u8; 4] = [1, 2, 3, 4];
        let b: [u8; 4] = [5, 6, 7, 8];
        unsafe {
            q.copy_from_ptr(3, a.as_ptr(), 4, 100);
            q.copy_from_ptr(3, b.as_ptr(), 4, 200);
        }
        assert_eq!(q.pop().unwrap().timestamp, 100);
        assert_eq!(q.pop().unwrap().timestamp, 200);
        assert!(q.pop().is_none());
    }

    #[test]
    fn stats_reflects_full_queue() {
        let q = VendorRxQueue::new();
        let buf = [0u8; 4];
        for _ in 0..VENDOR_RX_QUEUE_CAPACITY {
            unsafe {
                q.copy_from_ptr(3, buf.as_ptr(), 4, 0);
            }
        }
        assert!(!unsafe { q.copy_from_ptr(3, buf.as_ptr(), 4, 0) });
        assert!(q.stats().dropped > 0);
    }

    #[test]
    fn pop_after_full() {
        let q = VendorRxQueue::new();
        let buf = [0u8; 4];
        for _ in 0..VENDOR_RX_QUEUE_CAPACITY - 1 {
            unsafe {
                q.copy_from_ptr(1, buf.as_ptr(), 4, 0);
            }
        }
        assert!(q.pop().is_some());
        assert!(unsafe { q.copy_from_ptr(1, buf.as_ptr(), 4, 0) });
    }

    #[test]
    fn stats_separates_dropped_and_too_large() {
        let q = VendorRxQueue::new();
        unsafe {
            q.copy_from_ptr(1, core::ptr::null(), 4, 0);
        }
        assert_eq!(q.stats().too_large, 1);
        assert_eq!(q.stats().dropped, 0);

        let buf = [0u8; 4];
        for _ in 0..VENDOR_RX_QUEUE_CAPACITY - 1 {
            unsafe {
                q.copy_from_ptr(1, buf.as_ptr(), 4, 0);
            }
        }
        // 满队列后再入队触发 dropped
        unsafe {
            q.copy_from_ptr(1, buf.as_ptr(), 4, 0);
        }
        assert_eq!(q.stats().dropped, 1);
        assert_eq!(q.stats().too_large, 1);
    }
}
