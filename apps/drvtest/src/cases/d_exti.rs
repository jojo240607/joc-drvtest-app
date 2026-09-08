//! d_exti：外部中断（exti0 = PE5 → EXTI 线5 → EXTI9_5 / IRQ23）。
//!
//! exti 驱动的 open 只挂 ISR、不使能 NVIC（使能属于事件设备的 enable 阶段，
//! SDK 未上提事件接口）——因此本用例经 App 的 `irq_attach` 在同一 IRQ 线上再挂
//! 一个 App ISR：`app_slot_irq_attach` 内部会 irq_manager_enable 该线，从而
//! 打开 NVIC 掩码，驱动 ISR 与 App ISR 同时被派发（共享线多 handler 语义）。
//! 判据：TRIGGER 两次后 ①驱动计数推进 ≥2；②App ISR 收到信号量。

use core::ffi::c_void;
use core::ptr::null_mut;

use rtos_app_sdk::ioctl::*;
use rtos_app_sdk::irq;
use rtos_app_sdk::rtos::{msleep, tick_count, Semaphore};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// F407：EXTI 线5 → EXTI9_5_IRQn = 23。
const EXTI9_5_IRQ: u8 = 23;

/// App 自己的 EXTI ISR：给信号量（中断上下文，只做最快的事）。
static mut EXT_SEM: Semaphore = Semaphore::uninit();

extern "C" fn ext_isr(_ctx: *mut c_void) {
    unsafe {
        EXT_SEM.give();
    }
}

/// exti0：软件触发两次 → 驱动计数推进 ≥2，且 App ISR 收到信号量。
pub fn exti_trigger(_ctx: &mut Ctx) -> Verdict {
    unsafe {
        EXT_SEM.init();
    }
    let dev = match cap::open("exti0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 同一 EXTI9_5 线上挂 App ISR（经它打开 NVIC 掩码；与驱动 ISR 并存）。
    let rc = irq::attach_and_enable(EXTI9_5_IRQ, ext_isr, null_mut());
    if rc != 0 {
        return Verdict::Skip("irq_attach(EXTI9_5) 不可用");
    }

    let mut c0: u32 = 0;
    if dev.ioctl(EXTI_IOCTL_GET_COUNT, (&mut c0 as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_COUNT 失败");
    }

    if dev.ioctl(EXTI_IOCTL_TRIGGER, null_mut()) != 0 {
        return Verdict::Fail("TRIGGER#1 失败");
    }
    if dev.ioctl(EXTI_IOCTL_TRIGGER, null_mut()) != 0 {
        return Verdict::Fail("TRIGGER#2 失败");
    }

    // 等 App ISR 触发（信号量 0→1，限 1：只证明 ISR 跑过；两次触发由驱动计数核对）。
    // trywait 轮询带 2000tick 死线，避免异常时挂死用例。
    let t0 = tick_count();
    let mut app_got = 0;
    loop {
        if unsafe { EXT_SEM.trywait() } == 0 {
            app_got = 1;
            break;
        }
        if tick_count().wrapping_sub(t0) > 2000 {
            break;
        }
        msleep(2);
    }
    let _ = irq::disable(EXTI9_5_IRQ);

    let mut c1: u32 = 0;
    if dev.ioctl(EXTI_IOCTL_GET_COUNT, (&mut c1 as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_COUNT#2 失败");
    }

    if c1.wrapping_sub(c0) >= 2 && app_got == 1 {
        Verdict::Pass("exti0 软件触发 → 驱动+App 共享线 ISR 均收到（计数≥2）")
    } else {
        Verdict::Fail("EXTI 计数未推进或 App ISR 未触发")
    }
}
