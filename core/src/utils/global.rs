use core::cell::UnsafeCell;

/// 这个容器只允许主循环访问，ISR 不能碰
pub struct MainLoopGlobal<T> {
    inner: UnsafeCell<Option<T>>,
}

// SAFETY: 访问约束由单线程主循环保证
unsafe impl<T> Sync for MainLoopGlobal<T> {}

impl<T> MainLoopGlobal<T> {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    pub fn init(&self, value: T) {
        // SAFETY: 初始化发生在主循环语境中，不存在并发写
        unsafe {
            *self.inner.get() = Some(value);
        }
    }

    pub fn execute<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        // SAFETY: 主循环里不会出现可变别名
        unsafe {
            let opt = &mut *self.inner.get();
            opt.as_mut().map(f)
        }
    }
}
