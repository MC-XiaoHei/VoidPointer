use core::cell::UnsafeCell;

/// 只允许主循环访问，ISR 禁用
pub struct MainLoopGlobal<T> {
    inner: UnsafeCell<Option<T>>,
}

// SAFETY: 仅限非抢占主循环访问
unsafe impl<T> Sync for MainLoopGlobal<T> {}

impl<T> MainLoopGlobal<T> {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    pub fn init(&self, value: T) {
        // SAFETY: 主循环内无并发写
        unsafe {
            *self.inner.get() = Some(value);
        }
    }

    pub fn execute<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        // SAFETY: 主循环内不会产生可变别名
        unsafe {
            let opt = &mut *self.inner.get();
            opt.as_mut().map(f)
        }
    }
}
