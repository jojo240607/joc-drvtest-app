//! d_can：CAN 回环自测（数值正确）。
//!
//! can0 = CAN1，平台配置 loopback=1（BTR.LBKM）——控制器把自身 TX 帧回环到
//! RX FIFO，无需外部收发器/总线。判据：SEND 一帧已知数据 → RECV 回环帧
//! 逐字段比对（ID/扩展位/RTR/DLC/8 字节数据）+ 寄存器回读。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// can0：SEND_FRAME(0x123, 8 字节) → RECV_FRAME 回环帧内容一致。
pub fn can_loopback(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("can0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // 回读 BTR 确认回环模式（平台配置 loopback=1；若平台未配回环则跳过）
    let mut btr: u32 = 0;
    if dev.ioctl(CAN_IOCTL_GET_BTR, (&mut btr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_BTR 失败");
    }
    if btr & CAN_BTR_LBKM == 0 {
        return Verdict::Skip("can0 非回环模式（平台配置）");
    }

    // 发一帧：标准帧 id=0x123，8 字节已知数据
    let tx = CanFrame {
        id: 0x123,
        ext: 0,
        rtr: 0,
        dlc: 8,
        data: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88],
    };
    let mut t = tx;
    if dev.ioctl(CAN_IOCTL_SEND_FRAME, (&mut t as *mut CanFrame).cast()) != 0 {
        return Verdict::Fail("SEND_FRAME 失败");
    }

    // 收回环帧（loopback 立即回环；RECV 内部自带超时轮询，不会无限挂起）
    let mut rx = CanFrame { id: 0, ext: 0, rtr: 0, dlc: 0, data: [0; 8] };
    if dev.ioctl(CAN_IOCTL_RECV_FRAME, (&mut rx as *mut CanFrame).cast()) != 0 {
        return Verdict::Fail("RECV_FRAME 未收到回环帧");
    }
    if rx.id != tx.id || rx.ext != tx.ext || rx.rtr != tx.rtr || rx.dlc != tx.dlc
        || rx.data != tx.data
    {
        return Verdict::Fail("回环帧内容不一致");
    }

    // 寄存器回读（MCR/TSR 至少可读，且发送完成标志应置位）
    let mut mcr: u32 = 0;
    if dev.ioctl(CAN_IOCTL_GET_MCR, (&mut mcr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_MCR 失败");
    }
    let mut tsr: u32 = 0;
    if dev.ioctl(CAN_IOCTL_GET_TSR, (&mut tsr as *mut u32).cast()) != 0 {
        return Verdict::Fail("GET_TSR 失败");
    }
    if tsr & 0x02 == 0 {
        // TSR.TXOK0（bit1）：邮箱 0 发送成功（F407 TSR：RQCP0=bit0/TXOK0=bit1）
        return Verdict::Fail("TSR 无发送完成标志");
    }

    Verdict::Pass("can0 loopback 发→收回环帧数值一致")
}
