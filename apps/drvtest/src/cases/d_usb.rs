//! d_usb：USB OTG FS 设备控制器（无主机连接）。
//!
//! usb0 是 CDC 设备控制器（slave/FIFO 模式，无 DMA——规避 F407 ES0206 擦写
//! 损坏）。模拟器/真机此刻都无主机枚举，故判据分两层：
//! 1) open + 全局/设备寄存器回读（GINTSTS/GCCFG/DSTS/ADDRESS）；
//! 2) RUN_CTRL_SELFTEST——固件 USB 栈的 6 步合成控制传输自测（GET_DESCRIPTOR
//!    ×3 + GET/SET_LINE_CODING + SET_CONTROL_LINE_STATE），纯软件栈内完成，
//!    不依赖物理主机，是「USB 栈数值正确」的硬判据。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// usb0：open + 寄存器回读 + USB 栈自测。
pub fn usb_ioctl(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("usb0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 全局/设备寄存器回读（open 已完成 USBD_Init）
    let mut gintsts: u32 = 0;
    if dev.ioctl(USB_IOCTL_GET_GINTSTS, (&mut gintsts as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_GINTSTS 失败");
    }
    let mut gccfg: u32 = 0;
    if dev.ioctl(USB_IOCTL_GET_GCCFG, (&mut gccfg as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_GCCFG 失败");
    }
    let mut dsts: u32 = 0;
    if dev.ioctl(USB_IOCTL_GET_DSTS, (&mut dsts as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_DSTS 失败");
    }
    let mut addr: u32 = 0;
    if dev.ioctl(USB_IOCTL_GET_ADDRESS, (&mut addr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_ADDRESS 失败");
    }

    // 固件 USB 栈自测（合成控制传输序列；失败即栈有缺陷）
    if dev.ioctl(USB_IOCTL_RUN_CTRL_SELFTEST, core::ptr::null_mut()) != 0 {
        return Verdict::Fail("USB 栈自测失败");
    }

    Verdict::Pass("usb0 open + 寄存器回读 + USB 栈自测通过")
}
