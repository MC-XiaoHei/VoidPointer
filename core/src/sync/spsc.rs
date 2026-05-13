use core::cell::UnsafeCell;
use core::sync::atomic::Ordering;
use core::sync::atomic::compiler_fence;

/// SPSC 环形缓冲区
///
/// `CAP` 必须是 2 的幂
pub struct SpscQueue<T, const CAP: usize> {
    buf: UnsafeCell<[T; CAP]>,
    head: UnsafeCell<u32>,
    tail: UnsafeCell<u32>,
    dropped: UnsafeCell<u32>,
    drop_detail: UnsafeCell<u32>,
}

unsafe impl<T, const CAP: usize> Sync for SpscQueue<T, CAP> {}

impl<T: Copy, const CAP: usize> SpscQueue<T, CAP> {
    pub const fn from_array(buf: [T; CAP]) -> Self {
        Self {
            buf: UnsafeCell::new(buf),
            head: UnsafeCell::new(0),
            tail: UnsafeCell::new(0),
            dropped: UnsafeCell::new(0),
            drop_detail: UnsafeCell::new(0),
        }
    }

    /// 生产者入队。返回 false 表示缓冲区满
    pub fn push(&self, value: T) -> bool {
        let buf = unsafe { &mut *self.buf.get() };
        let head = unsafe { *self.head.get() };
        let tail = unsafe { *self.tail.get() };
        let next_head = (head + 1) & self.mask();

        if next_head == tail {
            unsafe { *self.dropped.get() += 1 };
            return false;
        }

        buf[head as usize] = value;
        compiler_fence(Ordering::Release);
        unsafe { *self.head.get() = next_head };
        true
    }

    /// 消费者出队
    pub fn pop(&self) -> Option<T> {
        let buf = unsafe { &*self.buf.get() };
        let head = unsafe { *self.head.get() };
        let tail = unsafe { *self.tail.get() };

        if head == tail {
            return None;
        }

        let value = buf[tail as usize];
        compiler_fence(Ordering::Acquire);
        unsafe { *self.tail.get() = (tail + 1) & self.mask() };
        Some(value)
    }

    pub fn is_empty(&self) -> bool {
        let head = unsafe { *self.head.get() };
        let tail = unsafe { *self.tail.get() };
        head == tail
    }

    /// 队列满导致的丢弃数
    pub fn dropped(&self) -> u32 {
        unsafe { *self.dropped.get() }
    }

    /// 其他原因导致的丢弃/拒绝计数（生产者写入）
    ///
    /// 入参 `n` 会加到累计值上。用于记录不经过 push 的拒绝（如数据超长）
    pub fn mark_drop_detail(&self, n: u32) {
        unsafe { *self.drop_detail.get() += n };
    }

    /// 获取额外丢弃计数
    pub fn drop_detail(&self) -> u32 {
        unsafe { *self.drop_detail.get() }
    }

    fn mask(&self) -> u32 {
        (CAP - 1) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled<const N: usize>() -> SpscQueue<u32, N> {
        SpscQueue::from_array([0u32; N])
    }

    #[test]
    fn pop_empty() {
        let q = filled::<4>();
        assert!(q.pop().is_none());
        assert!(q.is_empty());
    }

    #[test]
    fn push_then_pop() {
        let q = filled::<4>();
        assert!(q.push(42));
        assert!(!q.is_empty());
        assert_eq!(q.pop(), Some(42));
        assert!(q.is_empty());
    }

    #[test]
    fn fifo_order() {
        let q = filled::<4>();
        q.push(10);
        q.push(20);
        q.push(30);
        assert_eq!(q.pop(), Some(10));
        assert_eq!(q.pop(), Some(20));
        assert_eq!(q.pop(), Some(30));
    }

    #[test]
    fn full_rejects_and_counts_dropped() {
        let q = filled::<4>();
        assert!(q.push(1));
        assert!(q.push(2));
        assert!(q.push(3));
        assert!(!q.push(4));
        assert_eq!(q.dropped(), 1);
        assert!(!q.push(5));
        assert_eq!(q.dropped(), 2);
    }

    #[test]
    fn pop_after_full() {
        let q = filled::<4>();
        q.push(10);
        q.push(20);
        q.push(30);
        q.push(40);
        assert_eq!(q.pop(), Some(10));
        assert_eq!(q.pop(), Some(20));
    }

    #[test]
    fn push_after_pop_reuses_slot() {
        let q = filled::<4>();
        q.push(1);
        q.push(2);
        q.push(3);
        assert_eq!(q.pop(), Some(1));
        assert!(q.push(4));
        assert!(!q.is_empty());
    }

    #[test]
    fn wrap_around() {
        let q = filled::<4>();
        // 填到 cap-1，预留一个空位区分空/满
        q.push(1);
        q.push(2);
        q.push(3);
        // 全部弹出让 tail 追上 head
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), Some(3));
        // head == tail == 3，下一个入队写到 buf[3]
        assert!(q.push(10));
        assert_eq!(q.pop(), Some(10));
        // head == tail == 0，回绕到 buf[0]
        assert!(q.push(20));
        assert_eq!(q.pop(), Some(20));
    }

    #[test]
    fn wrap_around_multiple_cycles() {
        let q = filled::<4>();
        for round in 0..10 {
            for i in 0..3 {
                assert!(q.push(round * 100 + i));
            }
            for i in 0..3 {
                assert_eq!(q.pop(), Some(round * 100 + i));
            }
        }
        assert!(q.is_empty());
        assert_eq!(q.dropped(), 0);
    }

    #[test]
    fn capacity_2_basic() {
        let q = filled::<2>();
        assert!(q.push(1));
        assert!(!q.is_empty());
        assert_eq!(q.pop(), Some(1));
        assert!(q.is_empty());
    }

    #[test]
    fn capacity_2_one_slot_headroom() {
        let q = filled::<2>();
        assert!(q.push(1));
        assert!(!q.push(2));
        assert_eq!(q.dropped(), 1);
        assert_eq!(q.pop(), Some(1));
        assert!(q.push(2));
        assert_eq!(q.pop(), Some(2));
    }

    #[test]
    fn capacity_8_deep() {
        let q = filled::<8>();
        for i in 0..7 {
            assert!(q.push(i));
        }
        assert!(!q.push(99));
        assert_eq!(q.dropped(), 1);
        for i in 0..7 {
            assert_eq!(q.pop(), Some(i));
        }
        assert!(q.is_empty());
    }

    #[test]
    fn drop_detail_default_zero() {
        let q = filled::<4>();
        assert_eq!(q.drop_detail(), 0);
    }

    #[test]
    fn mark_drop_detail_accumulates() {
        let q = filled::<4>();
        q.mark_drop_detail(5);
        assert_eq!(q.drop_detail(), 5);
        q.mark_drop_detail(3);
        assert_eq!(q.drop_detail(), 8);
    }

    #[test]
    fn dropped_zero_on_empty_pop() {
        let q = filled::<4>();
        assert!(q.pop().is_none());
        assert_eq!(q.dropped(), 0);
    }

    #[test]
    fn dropped_not_counted_on_successful_push() {
        let q = filled::<4>();
        q.push(1);
        q.push(2);
        q.push(3);
        assert_eq!(q.dropped(), 0);
    }

    #[test]
    fn from_array_with_non_zero() {
        let q = SpscQueue::from_array([10u32, 20, 30, 40]);
        assert!(q.is_empty());
        q.push(1);
        assert_eq!(q.pop(), Some(1));
    }

    #[test]
    fn is_empty_after_push_pop_cycle() {
        let q = filled::<4>();
        assert!(q.is_empty());
        q.push(7);
        assert!(!q.is_empty());
        q.pop();
        assert!(q.is_empty());
    }

    #[test]
    fn push_returns_true_on_success() {
        let q = filled::<4>();
        assert!(q.push(1));
        assert!(q.push(2));
        assert!(q.push(3));
    }

    #[test]
    fn push_returns_false_when_full() {
        let q = filled::<4>();
        q.push(1);
        q.push(2);
        q.push(3);
        assert!(!q.push(4));
    }

    #[test]
    fn pop_returns_none_when_empty_after_use() {
        let q = filled::<4>();
        q.push(5);
        q.pop();
        assert!(q.pop().is_none());
    }

    #[test]
    fn dropped_and_drop_detail_independent() {
        let q = filled::<4>();
        // 填满队列触发 dropped
        q.push(1);
        q.push(2);
        q.push(3);
        q.push(4);
        assert_eq!(q.dropped(), 1);
        assert_eq!(q.drop_detail(), 0);
        // 额外计数
        q.mark_drop_detail(2);
        assert_eq!(q.dropped(), 1);
        assert_eq!(q.drop_detail(), 2);
    }
}
