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

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn get_returns_none_initially() {
        clear_current_attitude();
        assert!(get_current_attitude().is_none());
    }

    #[test]
    fn update_then_get() {
        clear_current_attitude();
        let raw = SflpGameRotationRaw { x: 0, y: 0, z: 0 };
        let result = update_current_attitude_from_raw(raw);
        assert!((result.w - 1.0).abs() < 1e-6);
        let cached = get_current_attitude();
        assert!(cached.is_some());
        assert!((cached.unwrap().w - 1.0).abs() < 1e-6);
    }

    #[test]
    fn clear_after_update() {
        clear_current_attitude();
        let raw = SflpGameRotationRaw {
            x: half::f16::from_f32(1.0).to_bits(),
            y: 0,
            z: 0,
        };
        update_current_attitude_from_raw(raw);
        assert!(get_current_attitude().is_some());
        clear_current_attitude();
        assert!(get_current_attitude().is_none());
    }
}
