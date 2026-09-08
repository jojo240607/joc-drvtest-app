//! d_timer_multi：timer1-13 实例级覆盖（timer0 已在 d_timer 覆盖）。
//! 判据：每个实例 open 成功 → ENABLE 后 20Hz 溢出计数推进 ≥2 → DISABLE。
//! IRQ 共享线（TIM1/TIM10→IRQ25、TIM8/TIM13→IRQ44）走多 handler 并存，无冲突。

use core::ptr::null_mut;

use rtos_app_sdk::ioctl::*;
use rtos_app_sdk::rtos::msleep;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// board 注册的 13 个未覆盖定时器实例（timer0=TIM2 已在 d_timer 覆盖）。
const TIMER_INSTANCES: [&str; 13] = [
    "timer1", "timer2", "timer3", "timer4", "timer5", "timer6", "timer7",
    "timer8", "timer9", "timer10", "timer11", "timer12", "timer13",
];

/// timer1-13：open + 20Hz 溢出计数推进（TIM→驱动 ISR 链路）。
pub fn timer_instances(_ctx: &mut Ctx) -> Verdict {
    for name in TIMER_INSTANCES {
        let dev = match cap::open(name) {
            Ok(d) => d,
            Err(_) => return Verdict::Fail("timer 实例 open 失败（无引脚依赖）"),
        };

        let mut ov0: u32 = 0;
        if dev.ioctl(TIMER_IOCTL_GET_OVERFLOWS, (&mut ov0 as *mut u32).cast()) != 0 {
            return Verdict::Fail("GET_OVERFLOWS 失败");
        }
        // 若前一实例残留 ENABLE（共享 IRQ 多 handler），本实例计数会从非 0 起跳；无妨。
        if dev.ioctl(TIMER_IOCTL_ENABLE, null_mut()) != 0 {
            return Verdict::Fail("TIMER_ENABLE 失败");
        }
        // 20Hz → 50ms/拍；160ms（>3 拍）保证即使使能相位贴边也 ≥2 次溢出。
        msleep(160);
        let mut ov1: u32 = 0;
        if dev.ioctl(TIMER_IOCTL_GET_OVERFLOWS, (&mut ov1 as *mut u32).cast()) != 0 {
            return Verdict::Fail("GET_OVERFLOWS#2 失败");
        }
        let _ = dev.ioctl(TIMER_IOCTL_DISABLE, null_mut());

        if ov1.wrapping_sub(ov0) < 2 {
            crate::info!("d_timer_multi", "{name} 溢出计数未推进 ov0={ov0} ov1={ov1}");
            return Verdict::Fail("溢出计数未推进（≥2）");
        }
    }
    Verdict::Pass("timer1-13 open + 20Hz 溢出计数推进正常（13 实例）")
}
