use core::cell::UnsafeCell;

const EVENT_QUEUE_CAPACITY: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeEvent {
    BleConnected {
        timestamp: u32,
    },
    BleInputReady {
        timestamp: u32,
    },
    BleDisconnected {
        reason: u8,
        timestamp: u32,
    },
    DongleConnected {
        timestamp: u32,
    },
    DongleDisconnected {
        reason: u8,
        timestamp: u32,
    },
    UsbStateChanged {
        state: u8,
        timestamp: u32,
    },
    ButtonExti {
        button_id: u8,
        level: u8,
        timestamp: u32,
    },
    ModeSwitchExti {
        level: u8,
        timestamp: u32,
    },
    DebounceTick {
        timestamp: u32,
    },
    EncoderExti {
        a_level: u8,
        b_level: u8,
        timestamp: u32,
    },
    ImuInt {
        timestamp: u32,
    },
    ImuSample {
        raw_x: u16,
        raw_y: u16,
        raw_z: u16,
        timestamp: u32,
    },
    ImuFifoDone {
        status: u8,
        dropped_count: u16,
        timestamp: u32,
    },
    HidSendDone {
        route: u8,
        status: u8,
        timestamp: u32,
    },
    VendorReportRx {
        route: u8,
        len: u16,
        timestamp: u32,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventQueueStats {
    pub dropped: u16,
}

struct EventQueueInner {
    buf: [Option<RuntimeEvent>; EVENT_QUEUE_CAPACITY],
    head: usize,
    tail: usize,
    len: usize,
    dropped: u16,
}

impl EventQueueInner {
    const fn new() -> Self {
        Self {
            buf: [None; EVENT_QUEUE_CAPACITY],
            head: 0,
            tail: 0,
            len: 0,
            dropped: 0,
        }
    }

    fn push(&mut self, event: RuntimeEvent) -> bool {
        if self.len == EVENT_QUEUE_CAPACITY {
            self.dropped = self.dropped.saturating_add(1);
            return false;
        }

        self.buf[self.tail] = Some(event);
        self.tail = (self.tail + 1) % EVENT_QUEUE_CAPACITY;
        self.len += 1;
        true
    }

    fn pop(&mut self) -> Option<RuntimeEvent> {
        if self.len == 0 {
            return None;
        }

        let event = self.buf[self.head].take();
        self.head = (self.head + 1) % EVENT_QUEUE_CAPACITY;
        self.len -= 1;
        event
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn stats(&self) -> EventQueueStats {
        EventQueueStats {
            dropped: self.dropped,
        }
    }
}

pub struct EventQueue {
    inner: UnsafeCell<EventQueueInner>,
}

// SAFETY: 所有访问由 CH585 单核临界区串行化
unsafe impl Sync for EventQueue {}

impl EventQueue {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(EventQueueInner::new()),
        }
    }

    pub fn push(&self, event: RuntimeEvent) -> bool {
        // ISR 内不能恢复 mstatus；这里只做短入队
        unsafe { (&mut *self.inner.get()).push(event) }
    }

    pub fn pop(&self) -> Option<RuntimeEvent> {
        unsafe { (&mut *self.inner.get()).pop() }
    }

    pub fn is_empty(&self) -> bool {
        unsafe { (&*self.inner.get()).is_empty() }
    }

    pub fn stats(&self) -> EventQueueStats {
        unsafe { (&*self.inner.get()).stats() }
    }
}
