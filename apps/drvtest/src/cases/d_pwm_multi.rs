//! d_pwm_multi：pwm1-4 实例级覆盖（pwm0=TIM3_CH1 已在 d_pwm 覆盖）。
//! 判据：每个实例 open 成功 → 周期回读 >0 → 50% 占空比回读 ≈ 周期一半（±5%）
//! → 通道使能/禁用。
//! 引脚无冲突（pwm1=TIM2_CH1_PA15、pwm2=TIM1_CH1_PA8、pwm3=TIM4_CH1_PD12、
//! pwm4=TIM12_CH1_PB14）。pwm4 为协调模式（复用 timer7 周期或 fallback 1kHz），
//! 周期回读恒 >0。

use core::ptr::null_mut;

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

const PWM_INSTANCES: [&str; 4] = ["pwm1", "pwm2", "pwm3", "pwm4"];

/// pwm1-4：周期/占空比 ioctl 读写一致 + 通道开关。
pub fn pwm_instances(_ctx: &mut Ctx) -> Verdict {
    for name in PWM_INSTANCES {
        let dev = match cap::open(name) {
            Ok(d) => d,
            Err(_) => return Verdict::Fail("pwm 实例 open 失败（引脚无冲突，应恒成功）"),
        };

        let mut period: u32 = 0;
        if dev.ioctl(PWM_IOCTL_GET_PERIOD_TICKS, (&mut period as *mut u32).cast()) != 0
            || period == 0
        {
            crate::info!("d_pwm_multi", "{name} GET_PERIOD_TICKS=0");
            return Verdict::Fail("GET_PERIOD_TICKS 失败/为 0");
        }

        let mut duty: i32 = 50;
        if dev.ioctl(PWM_IOCTL_SET_DUTY_PERCENT, (&mut duty as *mut i32).cast()) != 0 {
            return Verdict::Fail("SET_DUTY_PERCENT 失败");
        }
        let mut dticks: u32 = 0;
        if dev.ioctl(PWM_IOCTL_GET_DUTY_TICKS, (&mut dticks as *mut u32).cast()) != 0 {
            return Verdict::Fail("GET_DUTY_TICKS 失败");
        }
        let half = period / 2;
        let tol = period / 20; // 5%
        if dticks.abs_diff(half) > tol {
            crate::info!("d_pwm_multi", "{name} period={period} dticks={dticks} half={half} tol={tol}");
            return Verdict::Fail("50% 占空比回读偏差过大");
        }

        if dev.ioctl(PWM_IOCTL_ENABLE_CHANNEL, null_mut()) != 0 {
            return Verdict::Fail("ENABLE_CHANNEL 失败");
        }
        if dev.ioctl(PWM_IOCTL_DISABLE_CHANNEL, null_mut()) != 0 {
            return Verdict::Fail("DISABLE_CHANNEL 失败");
        }
    }
    Verdict::Pass("pwm1-4 周期/占空比 ioctl 读写一致 + 通道开关正常（4 实例）")
}
