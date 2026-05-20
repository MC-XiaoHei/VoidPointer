#![cfg_attr(coverage, coverage(off))]

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

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn new_is_none() {
        let g: MainLoopGlobal<u32> = MainLoopGlobal::new();
        assert_eq!(g.execute(|_| 42u32), None);
    }

    #[test]
    fn init_then_execute() {
        let g = MainLoopGlobal::new();
        g.init(42u32);
        assert_eq!(g.execute(|v| *v), Some(42));
    }

    #[test]
    fn execute_mutates_inner() {
        let g = MainLoopGlobal::new();
        g.init(10u32);
        let result = g.execute(|v| {
            *v += 5;
            *v
        });
        assert_eq!(result, Some(15));
        assert_eq!(g.execute(|v| *v), Some(15));
    }
}
