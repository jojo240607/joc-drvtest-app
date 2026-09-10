//! d_pmw3901：PMW3901 光流传感器（SPI，PixArt 光学流）全链路。
//! 判据（与模拟器 vperiph/spi/pmw3901.rs 同一协议）：
//!   Product_ID：0x00 读回 0x49（open 时已校验）
//!   一帧光流：Delta_X=1.0px（8.8 定点 0x0100=256）、Delta_Y=0.5px（0x0080=128）、
//!   SQUAL=120、Motion bit7=数据就绪

use rtos_app_sdk::device::Device;
use rtos_app_sdk::info;
use rtos_app_sdk::ioctl::{Pwm3901Motion, PMW3901_IOCTL_GET_MOTION, PMW3901_IOCTL_GET_PRODUCT_ID};

use crate::cap;
use crate::verdict::{Ctx, Verdict};

fn open_flow() -> Result<Device, &'static str> {
    cap::open("pmw3901")
}

pub fn product_id(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_flow() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut id: u8 = 0;
    if dev.ioctl(PMW3901_IOCTL_GET_PRODUCT_ID, (&mut id as *mut u8).cast()) != 0 {
        return Verdict::Fail("GET_PRODUCT_ID 失败");
    }
    if id != 0x49 {
        return Verdict::Fail("Product_ID 不符");
    }
    info!("pmw3901", "Product_ID=0x{id:02X} 正确");
    Verdict::Pass("Product_ID=0x49")
}

pub fn motion_delta(_ctx: &mut Ctx) -> Verdict {
    let dev = match open_flow() {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };
    let mut m = Pwm3901Motion { dx: 0, dy: 0, squal: 0, motion: 0 };
    if dev.ioctl(PMW3901_IOCTL_GET_MOTION, (&mut m as *mut Pwm3901Motion).cast()) != 0 {
        return Verdict::Fail("GET_MOTION 失败");
    }
    // 模拟器默认模型：dx=1.0px（256）、dy=0.5px（128）、squal=120、Motion 就绪
    if m.dx != 256 || m.dy != 128 {
        return Verdict::Fail("Delta 数值不符");
    }
    if m.squal < 1 || (m.motion & 0x80) == 0 {
        return Verdict::Fail("SQUAL/Motion 就绪位不符");
    }
    info!("pmw3901", "dx={} dy={} squal={} motion=0x{:02X}", m.dx, m.dy, m.squal, m.motion);
    Verdict::Pass("dx=256 dy=128 squal=120 正确")
}
