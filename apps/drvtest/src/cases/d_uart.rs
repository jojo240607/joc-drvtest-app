//! d_uart：串口驱动。uart0 是控制台（g_console 已 open，App 只 get 不 open）；
//! uart1/2/3 可 open，但模拟器上 DMA TX 可能永久阻塞 → 只测 open + ioctl。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// uart0 DMA TX 真机判据：`write` 走 `uart_dma_write`（DMA2_Stream7_CH4 把内存
/// 模式串搬运到 USART1_DR，DMA TC 中断确认完成），返回长度必须等于发送长度。
/// 模式串带唯一标记 `DRVTEST-DMA-TX-REAL` + 递增字节模式——宿主在虚拟主机
/// （console/终端）捕获 TX 字节流做字节级比对，证明数据**真实从 TX 发出**
/// （到达终端侧），而非仅写入数据寄存器。
pub fn uart0_dma_tx_real(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::get("uart0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 48 字节模式：18 字节标记头 + 30 字节递增字节（0x10, 0x17, 0x1E, ...）。
    let mut msg = [0u8; 48];
    let hdr = b"DRVTEST-DMA-TX-REAL:";
    msg[..hdr.len()].copy_from_slice(hdr);
    for (i, b) in msg.iter_mut().enumerate().skip(hdr.len()) {
        *b = (i as u8).wrapping_mul(7).wrapping_add(0x10);
    }
    let rc = dev.write(&msg);
    if rc != msg.len() as i32 {
        return Verdict::Fail("uart0 DMA TX 返回长度不符");
    }
    Verdict::Pass("uart0 DMA TX 完成（返回长度一致，宿主 console 捕获比对）")
}

/// uart0：写回显 + 波特率 ioctl 往返 + 非阻塞读（控制台路径）。
pub fn uart0_console(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::get("uart0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 写回显（控制台可见一行）。
    let msg = b"DRVTEST uart0 write ok\r\n";
    let rc = dev.write(msg);
    if rc != msg.len() as i32 {
        return Verdict::Fail("uart0 write 返回长度不符");
    }

    // 波特率 ioctl：get → set(原值) → get 往返一致。
    let mut baud: u32 = 0;
    if dev.ioctl(UART_IOCTL_GET_BAUDRATE, (&mut baud as *mut u32).cast()) != 0 || baud == 0 {
        return Verdict::Fail("GET_BAUDRATE 失败/为 0");
    }
    let mut keep = baud;
    if dev.ioctl(UART_IOCTL_SET_BAUDRATE, (&mut keep as *mut u32).cast()) != 0 {
        return Verdict::Fail("SET_BAUDRATE 失败");
    }
    let mut baud2: u32 = 0;
    if dev.ioctl(UART_IOCTL_GET_BAUDRATE, (&mut baud2 as *mut u32).cast()) != 0 || baud2 != keep {
        return Verdict::Fail("波特率往返不一致");
    }

    // 非阻塞读（无输入 → 返回 0 或 <=len）。
    let mut buf = [0u8; 8];
    let rrc = dev.read(&mut buf);
    if rrc < 0 || rrc > 8 {
        return Verdict::Fail("read 返回值异常");
    }

    Verdict::Pass("uart0 写/波特率 ioctl/非阻塞读正常")
}

/// uart1/2/3：open + 波特率 ioctl（模拟器上 TX DMA 可能阻塞 → 不写）。
pub fn uart_others(_ctx: &mut Ctx) -> Verdict {
    let mut ok: u32 = 0;
    for name in ["uart1", "uart2", "uart3"] {
        let dev = match cap::open(name) {
            Ok(d) => d,
            Err(r) => return Verdict::Skip(r),
        };
        let mut baud: u32 = 0;
        if dev.ioctl(UART_IOCTL_GET_BAUDRATE, (&mut baud as *mut u32).cast()) == 0 && baud > 0 {
            ok += 1;
        } else {
            return Verdict::Fail("uart 波特率 ioctl 失败");
        }
    }
    if ok == 3 {
        Verdict::Pass("uart1/2/3 open+波特率 ioctl 正常（TX 写路径模拟器跳过）")
    } else {
        Verdict::Fail("uart1/2/3 部分失败")
    }
}
