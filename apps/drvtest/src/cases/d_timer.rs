//! d_timer：通用定时器（timer0 = TIM2 @20Hz）+ App ISR 注册（irq_attach）。
//! 判据：
//!   1) 驱动 ISR 路径：ENABLE 后溢出计数推进（TIM 中断 → 驱动 ISR → 计数）；
//!   2) App ISR 路径：irq_attach 到同一条 TIM2_UP 线，App 自己的 ISR 收到溢出
//!      给信号量（irq_dispatch 依次调用线上全部处理器，与驱动 ISR 并存）。

use core::ffi::c_void;
use core::ptr::null_mut;

use rtos_app_sdk::ioctl::*;
use rtos_app_sdk::irq;
use rtos_app_sdk::rtos::{msleep, tick_count, Semaphore};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// F407：TIM2 更新中断号（stm32f407xx.h IRQn_Type，TIM2=28）。
const TIM2_UP_IRQ: u8 = 28;

/// App 自己的 TIM2 溢出 ISR：给信号量（中断上下文，只做最快的事）。
static mut TIM_SEM: Semaphore = Semaphore::uninit();

extern "C" fn tim2_isr(_ctx: *mut c_void) {
    unsafe {
        TIM_SEM.give();
    }
}

/// 用例 1：timer0 ENABLE 后溢出计数推进（驱动 ISR 计数，20Hz → 120ms ≈ 2 次）。
pub fn timer_overflow(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("timer0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    let mut ov0: u32 = 0;
    if dev.ioctl(TIMER_IOCTL_GET_OVERFLOWS, (&mut ov0 as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_OVERFLOWS 失败");
    }
    if dev.ioctl(TIMER_IOCTL_ENABLE, null_mut()) != 0 {
        return Verdict::Fail("TIMER_ENABLE 失败");
    }
    msleep(120);
    let mut ov1: u32 = 0;
    if dev.ioctl(TIMER_IOCTL_GET_OVERFLOWS, (&mut ov1 as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_OVERFLOWS#2 失败");
    }
    let _ = dev.ioctl(TIMER_IOCTL_DISABLE, null_mut());

    if ov1.wrapping_sub(ov0) >= 2 {
        Verdict::Pass("timer0 20Hz 溢出计数推进（TIM→驱动 ISR 链路）")
    } else {
        Verdict::Fail("溢出计数未推进")
    }
}

/// 用例 2：App 经 irq_attach 注册自己的 TIM2_UP ISR，收到溢出给信号量。
pub fn timer_app_isr(_ctx: &mut Ctx) -> Verdict {
    unsafe {
        TIM_SEM.init();
    }
    let dev = match cap::open("timer0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let rc = irq::attach_and_enable(TIM2_UP_IRQ, tim2_isr, null_mut());
    if rc != 0 {
        return Verdict::Skip("irq_attach 不可用");
    }

    if dev.ioctl(TIMER_IOCTL_ENABLE, null_mut()) != 0 {
        let _ = irq::disable(TIM2_UP_IRQ);
        return Verdict::Fail("TIMER_ENABLE 失败");
    }

    // 等 App ISR 给信号量（20Hz → 50ms 一拍；预算 2000ms 内必到）。
    let t0 = tick_count();
    unsafe {
        TIM_SEM.wait();
    }
    let dt = tick_count().wrapping_sub(t0);
    let _ = dev.ioctl(TIMER_IOCTL_DISABLE, null_mut());
    let _ = irq::disable(TIM2_UP_IRQ);

    if dt <= 2000 {
        Verdict::Pass("App ISR 经 irq_attach 收到 TIM2 溢出")
    } else {
        Verdict::Fail("App ISR 未在预算内触发")
    }
}
