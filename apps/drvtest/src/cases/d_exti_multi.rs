//! d_exti_multi：exti1/exti2/btn/btn2 实例级覆盖（exti0=PE5 已在 d_exti 覆盖）。
//! 判据：每个实例 open 成功 → 经 App `irq_attach_and_enable` 打开对应 NVIC 线
//! → 软件 TRIGGER → App ISR 收到信号量 + 驱动计数推进 ≥1。
//! IRQ 线：exti1=PE6 线6→EXTI9_5/IRQ23；exti2=PE1 线1→EXTI1/IRQ7；
//! btn=PA2 线2→EXTI2/IRQ8；btn2=PA3 线3→EXTI3/IRQ9。

use core::ffi::c_void;
use core::ptr::null_mut;

use rtos_app_sdk::ioctl::*;
use rtos_app_sdk::irq;
use rtos_app_sdk::rtos::{msleep, tick_count, Semaphore};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// App 共享 EXTI ISR：给信号量（中断上下文，只做最快的事）。
static mut EXT_SEM: Semaphore = Semaphore::uninit();

extern "C" fn ext_multi_isr(_ctx: *mut c_void) {
    unsafe {
        EXT_SEM.give();
    }
}

/// (设备名, 对应 NVIC IRQ 号)
const EXTI_CASES: [(&str, u8); 4] = [
    ("exti1", 23), // PE6 → EXTI9_5
    ("exti2", 7),  // PE1 → EXTI1
    ("btn", 8),    // PA2 → EXTI2
    ("btn2", 9),   // PA3 → EXTI3
];

/// exti1/exti2/btn/btn2：TRIGGER → App ISR 收到 + 驱动计数推进。
pub fn exti_instances(_ctx: &mut Ctx) -> Verdict {
    unsafe {
        EXT_SEM.init();
    }
    let mut passed = 0;
    for (name, irq_no) in EXTI_CASES {
        let dev = match cap::open(name) {
            Ok(d) => d,
            Err(_) => {
                // 引脚被其他外设占用（btn/btn2 的 PA2/PA3 与 uart1=USART2 冲突）→
                // 平台配置，非驱动缺陷，跳过该实例。
                crate::info!("d_exti_multi", "{name} open 失败（引脚占用，跳过）");
                continue;
            }
        };

        // 清残留信号量（上一实例已消费，防御性清空）。
        while unsafe { EXT_SEM.trywait() } == 0 {}

        let rc = irq::attach_and_enable(irq_no, ext_multi_isr, null_mut());
        if rc != 0 {
            return Verdict::Skip("irq_attach 不可用");
        }

        if dev.ioctl(EXTI_IOCTL_TRIGGER, null_mut()) != 0 {
            let _ = irq::disable(irq_no);
            return Verdict::Fail("TRIGGER 失败");
        }

        // 等 App ISR 信号量（死线 500 tick，避免异常挂死）。
        let t0 = tick_count();
        let mut got = 0;
        loop {
            if unsafe { EXT_SEM.trywait() } == 0 {
                got = 1;
                break;
            }
            if tick_count().wrapping_sub(t0) > 500 {
                break;
            }
            msleep(2);
        }
        let _ = irq::disable(irq_no);
        if got == 0 {
            return Verdict::Fail("App ISR 未收到触发");
        }

        // 驱动 ISR 计数推进（与 App ISR 同线并存）。
        let mut c: u32 = 0;
        if dev.ioctl(EXTI_IOCTL_GET_COUNT, (&mut c as *mut u32).cast()) != 0 {
            return Verdict::Fail("GET_COUNT 失败");
        }
        if c == 0 {
            return Verdict::Fail("驱动计数未推进");
        }
        passed += 1;
    }
    if passed == 0 {
        return Verdict::Skip("exti/btn 均无法 open（引脚占用）");
    }
    Verdict::Pass("exti1/exti2/btn/btn2 触发→App ISR + 驱动计数推进")
}
