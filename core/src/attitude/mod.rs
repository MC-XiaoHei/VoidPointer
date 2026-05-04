use crate::attitude::types::{AttitudeData, SflpGameRotationRaw};
use core::cell::UnsafeCell;

pub mod types;

struct AttitudeCache {
    current: UnsafeCell<Option<AttitudeData>>,
}

// SAFETY: 这个缓存只在主循环 bottom-half 中读写
unsafe impl Sync for AttitudeCache {}

impl AttitudeCache {
    const fn new() -> Self {
        Self {
            current: UnsafeCell::new(None),
        }
    }

    fn set(&self, attitude: AttitudeData) {
        unsafe {
            *self.current.get() = Some(attitude);
        }
    }

    fn get(&self) -> Option<AttitudeData> {
        unsafe { *self.current.get() }
    }

    fn clear(&self) {
        unsafe {
            *self.current.get() = None;
        }
    }
}

static ATTITUDE_CACHE: AttitudeCache = AttitudeCache::new();

#[inline]
pub fn get_current_attitude() -> Option<AttitudeData> {
    ATTITUDE_CACHE.get()
}

#[inline]
pub fn update_current_attitude_from_raw(raw: SflpGameRotationRaw) -> AttitudeData {
    let attitude = AttitudeData::from(raw);
    ATTITUDE_CACHE.set(attitude);
    attitude
}

#[inline]
pub fn clear_current_attitude() {
    ATTITUDE_CACHE.clear();
}
