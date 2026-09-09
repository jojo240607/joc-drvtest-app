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

/// usb0 真实主机通信：与虚拟主机（cfg-run 宿主注入）两阶段握手——
/// 1) 写 READY 标记（uart0 console）→ 宿主注入总线复位 + 标准枚举
///    （GET_DESCRIPTOR×2 / SET_ADDRESS / SET_CONFIGURATION）；
/// 2) 轮询 USB_IOCTL_CONNECTED 直到主机枚举完成（connected=1）；
/// 3) 写 ENUM-OK 标记 → 宿主经 OUT EP1 注入 64B 模式数据（0x55+i）；
/// 4) dev.read 收全 64B → 逐字节数值比对。
/// 判据“数值正确”：收全 + 每字节 == 0x55+i——真实主机通信链路（枚举 +
/// 批量 OUT 数据面）在 App 层感知。
pub fn usb_host_comms(_ctx: &mut Ctx) -> Verdict {
    // READY 标记经 uart0（控制台）上报宿主。
    if let Ok(u0) = cap::get("uart0") {
        let _ = u0.write(b"DRVTEST-USB-HOST-READY\r\n");
    }
    let dev = match cap::get("usb0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 阶段 1：轮询等待主机枚举完成（cfg-run 检测 READY 后注入枚举序列）。
    let mut connected: u32 = 0;
    let mut n: u32 = 0;
    loop {
        if dev.ioctl(USB_IOCTL_CONNECTED, (&mut connected as *mut u32).cast()) != 0 {
            return Verdict::Fail("USB_IOCTL_CONNECTED 失败");
        }
        if connected != 0 {
            break;
        }
        n += 1;
        if n > 12_000_000 {
            return Verdict::Fail("等待主机枚举超时（connected 未置位）");
        }
    }

    // 阶段 2：通知宿主注入 OUT 数据。
    if let Ok(u0) = cap::get("uart0") {
        let _ = u0.write(b"DRVTEST-USB-ENUM-OK\r\n");
    }

    // 阶段 3：读主机 OUT 数据（64B pattern 0x55+i 进 USB RX ring）。
    let mut got = [0u8; 64];
    let mut got_n = 0usize;
    let mut m: u32 = 0;
    while got_n < 64 && m < 12_000_000 {
        let rc = dev.read(&mut got[got_n..]);
        if rc > 0 {
            got_n += rc as usize;
        }
        m += 1;
    }
    if got_n != 64 {
        return Verdict::Fail("主机 OUT 数据未收全");
    }
    for (i, b) in got.iter().enumerate() {
        if *b != 0x55u8.wrapping_add(i as u8) {
            return Verdict::Fail("主机 OUT 数据数值不符");
        }
    }
    Verdict::Pass("usb0 主机枚举 + OUT 64B 模式数据数值一致（真实主机通信）")
}
