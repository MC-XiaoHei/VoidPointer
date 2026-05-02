use core::arch::asm;

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
#[inline(always)]
pub fn interrupt_free<R>(f: impl FnOnce() -> R) -> R {
    let mstatus: usize;
    unsafe {
        asm!("csrr {0}, mstatus", out(reg) mstatus, options(nomem, nostack));
        asm!("csrci mstatus, 8", options(nomem, nostack));
    }

    let result = f();

    unsafe {
        asm!("csrw mstatus, {0}", in(reg) mstatus, options(nomem, nostack));
    }

    result
}

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
#[inline(always)]
pub fn interrupt_free<R>(f: impl FnOnce() -> R) -> R {
    f()
}
