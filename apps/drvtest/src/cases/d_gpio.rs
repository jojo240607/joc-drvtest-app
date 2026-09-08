//! d_gpio：GPIO 引脚驱动。用输出脚做 写→读回→toggle 往返（模拟器 IDR 自环
//! 读回 ODR）；输入脚（gpioc0/gpioe3）只做 open + 读（无外部注入时为 0）。

use core::ptr::null_mut;

use rtos_app_sdk::ioctl::GPIO_IOCTL_TOGGLE;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// 输出脚写读往返 + toggle（led/gpiob0/gpiod13 均为输出脚）。
pub fn gpio_output_roundtrip(_ctx: &mut Ctx) -> Verdict {
    for name in ["led", "gpiob0", "gpiod13"] {
        let dev = match cap::open(name) {
            Ok(d) => d,
            Err(r) => return Verdict::Skip(r),
        };
        // write 1 → read 1
        if dev.write(&[1u8]) != 1 {
            return Verdict::Fail("gpio write(1) 失败");
        }
        let mut r = [0u8; 1];
        if dev.read(&mut r) != 1 || r[0] != 1 {
            return Verdict::Fail("gpio 写1读回≠1");
        }
        // write 0 → read 0
        if dev.write(&[0u8]) != 1 {
            return Verdict::Fail("gpio write(0) 失败");
        }
        if dev.read(&mut r) != 1 || r[0] != 0 {
            return Verdict::Fail("gpio 写0读回≠0");
        }
        // toggle → read 1
        if dev.ioctl(GPIO_IOCTL_TOGGLE, null_mut()) != 0 {
            return Verdict::Fail("gpio toggle 失败");
        }
        if dev.read(&mut r) != 1 || r[0] != 1 {
            return Verdict::Fail("gpio toggle 后读回≠1");
        }
        // 还原 0
        let _ = dev.write(&[0u8]);
    }
    Verdict::Pass("led/gpiob0/gpiod13 写读往返 + toggle 正常")
}

/// 输入脚 open + 读（无外部注入时读回 0/1 均合法，只证明可访问）。
pub fn gpio_input_probe(_ctx: &mut Ctx) -> Verdict {
    for name in ["gpioc0", "gpioe3"] {
        let dev = match cap::open(name) {
            Ok(d) => d,
            Err(r) => return Verdict::Skip(r),
        };
        let mut r = [0xFFu8; 1];
        if dev.read(&mut r) != 1 {
            return Verdict::Fail("输入脚 read 失败");
        }
    }
    Verdict::Pass("gpioc0/gpioe3 输入脚 open+read 正常")
}
