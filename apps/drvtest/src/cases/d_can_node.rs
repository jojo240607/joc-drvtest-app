//! d_can_node：CAN 总线虚拟节点（电机控制器 0x201）全链路。
//! 判据（与模拟器 vperiph/can/node.rs 同一协议）：
//!   发读状态命令 [0x01] → 节点回状态帧 [rpm_hi=0x17, rpm_lo=0x70, temp=40, status=0x03]
//!   （rpm=6000=0x1770，与 ESC 电机转速口径一致）
//! 注意：can0 回环模式下自回环帧先入 FIFO，循环读直到命中节点响应帧。

use rtos_app_sdk::device::Device;
use rtos_app_sdk::info;
use rtos_app_sdk::ioctl::{CanFrame, CAN_IOCTL_RECV_FRAME, CAN_IOCTL_SEND_FRAME};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

pub fn motor_ctrl_response(_ctx: &mut Ctx) -> Verdict {
    let dev: Device = match cap::open("can0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    // 读状态命令 → 0x201 电机控制器节点
    let mut tx = CanFrame { id: 0x201, ext: 0, rtr: 0, dlc: 1, data: [0x01, 0, 0, 0, 0, 0, 0, 0] };
    if dev.ioctl(CAN_IOCTL_SEND_FRAME, (&mut tx as *mut CanFrame).cast()) != 0 {
        return Verdict::Fail("SEND_FRAME 失败");
    }
    // 自回环帧（data[0]=0x01）与节点响应帧（data[0]=0x17）都在 RX FIFO
    for _ in 0..4 {
        let mut rx = CanFrame { id: 0, ext: 0, rtr: 0, dlc: 0, data: [0; 8] };
        if dev.ioctl(CAN_IOCTL_RECV_FRAME, (&mut rx as *mut CanFrame).cast()) != 0 {
            continue;
        }
        if rx.id != 0x201 || rx.dlc != 4 || rx.data[0] != 0x17 {
            continue; // 自回环帧或其他帧：跳过
        }
        let rpm = ((rx.data[0] as u16) << 8) | rx.data[1] as u16;
        if rpm != 6000 || rx.data[2] != 40 || rx.data[3] != 0x03 {
            return Verdict::Fail("节点状态帧数值不符");
        }
        info!("can_node", "0x201 节点 rpm={rpm} temp={} status=0x{:02X}", rx.data[2], rx.data[3]);
        return Verdict::Pass("CAN 节点 0x201 回 rpm=6000/temp=40/status=0x03");
    }
    Verdict::Fail("未收到 0x201 节点响应帧")
}
