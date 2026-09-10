//! d_esc：ESC 电调 + BLDC 无刷电机虚拟外设（模拟器侧）全链路。
//!
//! PWM 路径：pwm0（TIM3_CH1 @400Hz/84MHz，周期 210000 ticks = 2500μs）
//!   60% 占空比 → 脉宽 1500μs → 电调量 1000（1~2ms 脉宽映射 0~2000）；
//! DShot 路径：dshot0（GPIOE_10 bit-bang DShot300）发送油门 1000 → 电调量 1000。
//!
//! 模拟器 ESC 虚拟外设（vperiph/esc.rs）订阅 TimPwm/GpioLevel 事件解码，
//! 宿主测试 x_drvtest 校验解码出的电调量/转速数值（本用例只负责产生信号）。

use rtos_app_sdk::device::Device;
use rtos_app_sdk::info;
use rtos_app_sdk::ioctl::{
    DSHOT_IOCTL_SEND, PWM_IOCTL_ENABLE_CHANNEL, PWM_IOCTL_SET_DUTY_PERCENT,
};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// PWM 电调输入：pwm0 60% 占空比 → 脉宽 1500μs → 电调量 1000
pub fn pwm_throttle(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("pwm0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    // 60% 占空比：400Hz 周期 2500μs → 脉宽 1500μs → 电调量 1000
    let mut duty: i32 = 60;
    if dev.ioctl(PWM_IOCTL_SET_DUTY_PERCENT, (&mut duty as *mut i32).cast()) != 0 {
        return Verdict::Fail("SET_DUTY_PERCENT 失败");
    }
    if dev.ioctl(PWM_IOCTL_ENABLE_CHANNEL, core::ptr::null_mut()) != 0 {
        return Verdict::Fail("ENABLE_CHANNEL 失败");
    }
    info!("esc", "pwm0 60%% duty → 脉宽 1500μs → 电调量 1000（模拟器 ESC 解码校验）");
    Verdict::Pass("pwm0 60% duty 已输出（电调量 1000，宿主机校验）")
}

/// DShot 电调输入：dshot0 发送油门 1000（16 位帧含 CRC4）
pub fn dshot_throttle(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("dshot0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut throttle: u16 = 1000;
    if dev.ioctl(DSHOT_IOCTL_SEND, (&mut throttle as *mut u16).cast()) != 0 {
        return Verdict::Fail("DSHOT_IOCTL_SEND 失败");
    }
    info!("esc", "dshot0 已发送油门 1000（模拟器 ESC 解码+CRC 校验）");
    Verdict::Pass("dshot0 油门 1000 已发送（电调量 1000，宿主机校验）")
}
