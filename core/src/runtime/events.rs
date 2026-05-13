use core::cell::UnsafeCell;
use core::sync::atomic::Ordering;
use core::sync::atomic::compiler_fence;

const EVENT_QUEUE_CAPACITY: usize = 16;
const EVENT_QUEUE_MASK: u32 = (EVENT_QUEUE_CAPACITY - 1) as u32;

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

struct EventQueueInner {
    buf: [RuntimeEvent; EVENT_QUEUE_CAPACITY],
    /// 仅 ISR 写入
    head: u32,
    /// 仅主循环写入
    tail: u32,
    /// 仅 ISR 写入
    dropped: u32,
}

pub struct EventQueue {
    inner: UnsafeCell<EventQueueInner>,
}

unsafe impl Sync for EventQueue {}

impl EventQueue {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(EventQueueInner {
                buf: [RuntimeEvent::VendorReportRx {
                    route: 0,
                    len: 0,
                    timestamp: 0,
                }; EVENT_QUEUE_CAPACITY],
                head: 0,
                tail: 0,
                dropped: 0,
            }),
        }
    }

    /// 只能在 ISR 上下文调用
    pub fn push(&self, event: RuntimeEvent) -> bool {
        let inner = unsafe { &mut *self.inner.get() };
        let next_head = (inner.head + 1) & EVENT_QUEUE_MASK;

        if next_head == inner.tail {
            inner.dropped += 1;
            return false;
        }

        inner.buf[inner.head as usize] = event;
        compiler_fence(Ordering::Release);
        inner.head = next_head;
        true
    }

    /// 只能在主循环上下文调用
    pub fn pop(&self) -> Option<RuntimeEvent> {
        let inner = unsafe { &mut *self.inner.get() };

        if inner.head == inner.tail {
            return None;
        }

        let event = inner.buf[inner.tail as usize];
        compiler_fence(Ordering::Acquire);
        inner.tail = (inner.tail + 1) & EVENT_QUEUE_MASK;
        Some(event)
    }

    pub fn is_empty(&self) -> bool {
        let inner = unsafe { &*self.inner.get() };
        inner.head == inner.tail
    }

    pub fn stats(&self) -> EventQueueStats {
        let inner = unsafe { &*self.inner.get() };
        EventQueueStats {
            dropped: inner.dropped,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventQueueStats {
    pub dropped: u32,
}
