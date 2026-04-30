use core::cell::UnsafeCell;

/// Core Contract: Only call within the main loop. NEVER use in an ISR!
pub struct MainLoopGlobal<T> {
    inner: UnsafeCell<Option<T>>,
}

// SAFETY: Thread-safe (Sync) ONLY if strictly confined to the non-preemptible main loop.
unsafe impl<T> Sync for MainLoopGlobal<T> {}

impl<T> MainLoopGlobal<T> {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    pub fn init(&self, value: T) {
        // SAFETY: Main loop execution prevents concurrent mutation.
        unsafe {
            *self.inner.get() = Some(value);
        }
    }

    pub fn execute<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        // SAFETY: Main loop execution guarantees no simultaneous `&mut T` aliasing.
        unsafe {
            let opt = &mut *self.inner.get();
            opt.as_mut().map(f)
        }
    }
}
