use crate::sync::spsc::SpscQueue;

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

pub struct EventQueue {
    inner: SpscQueue<RuntimeEvent, EVENT_QUEUE_CAPACITY>,
}

impl EventQueue {
    pub const fn new() -> Self {
        Self {
            inner: SpscQueue::from_array(
                [RuntimeEvent::VendorReportRx {
                    route: 0,
                    len: 0,
                    timestamp: 0,
                }; EVENT_QUEUE_CAPACITY],
            ),
        }
    }

    pub fn push(&self, event: RuntimeEvent) -> bool {
        self.inner.push(event)
    }

    pub fn pop(&self) -> Option<RuntimeEvent> {
        self.inner.pop()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn stats(&self) -> EventQueueStats {
        EventQueueStats {
            dropped: self.inner.dropped(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EventQueueStats {
    pub dropped: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(ts: u32) -> RuntimeEvent {
        RuntimeEvent::DebounceTick { timestamp: ts }
    }

    #[test]
    fn new_is_empty() {
        let q = EventQueue::new();
        assert!(q.pop().is_none());
        assert!(q.is_empty());
        assert_eq!(q.stats().dropped, 0);
    }

    #[test]
    fn push_then_pop() {
        let q = EventQueue::new();
        assert!(q.push(event(1)));
        assert!(!q.is_empty());
        assert_eq!(q.pop(), Some(event(1)));
        assert!(q.is_empty());
    }

    #[test]
    fn fifo_order() {
        let q = EventQueue::new();
        q.push(event(10));
        q.push(event(20));
        q.push(event(30));
        assert_eq!(q.pop(), Some(event(10)));
        assert_eq!(q.pop(), Some(event(20)));
        assert_eq!(q.pop(), Some(event(30)));
    }

    #[test]
    fn full_rejects_and_counts_dropped() {
        let q = EventQueue::new();
        let cap = 16usize;
        for i in 0..cap - 1 {
            assert!(q.push(event(i as u32)));
        }
        assert!(!q.push(event(99)));
        assert_eq!(q.stats().dropped, 1);
    }

    #[test]
    fn pop_after_full() {
        let q = EventQueue::new();
        for i in 0..15 {
            q.push(event(i));
        }
        assert_eq!(q.pop(), Some(event(0)));
        assert!(q.push(event(99)));
        assert_eq!(q.pop(), Some(event(1)));
    }

    #[test]
    fn wrap_around() {
        let q = EventQueue::new();
        for i in 0..15 {
            q.push(event(i));
        }
        for i in 0..15 {
            assert_eq!(q.pop(), Some(event(i as u32)));
        }
        for i in 100..114 {
            assert!(q.push(event(i)));
        }
        for i in 100..114 {
            assert_eq!(q.pop(), Some(event(i as u32)));
        }
    }

    #[test]
    fn stats_dropped_no_false_positive() {
        let q = EventQueue::new();
        q.push(event(1));
        q.push(event(2));
        assert_eq!(q.stats().dropped, 0);
        q.pop();
        q.pop();
        assert_eq!(q.stats().dropped, 0);
    }

    #[test]
    fn is_empty_edge_cases() {
        let q = EventQueue::new();
        assert!(q.is_empty());
        q.push(event(1));
        assert!(!q.is_empty());
        q.pop();
        assert!(q.is_empty());
        q.push(event(2));
        assert!(!q.is_empty());
    }
}
