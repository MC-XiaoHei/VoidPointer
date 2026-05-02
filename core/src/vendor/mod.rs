use crate::utils::critical::interrupt_free;
use core::cell::UnsafeCell;
use core::ptr;

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

pub struct VendorRuntime {
    pending_rx: bool,
    last_route: u8,
    last_len: u16,
    last_timestamp: u32,
    processed_count: u16,
}

impl VendorRuntime {
    pub fn new() -> Self {
        Self {
            pending_rx: false,
            last_route: 0,
            last_len: 0,
            last_timestamp: 0,
            processed_count: 0,
        }
    }

    pub fn mark_rx_pending(&mut self) {
        self.pending_rx = true;
    }

    pub fn poll(&mut self) {
        while let Some(report) = VENDOR_RX_QUEUE.pop() {
            self.last_route = report.route;
            self.last_len = report.len;
            self.last_timestamp = report.timestamp;
            self.processed_count = self.processed_count.saturating_add(1);
        }

        self.pending_rx = false;
    }
}

impl Default for VendorRuntime {
    fn default() -> Self {
        Self::new()
    }
}
