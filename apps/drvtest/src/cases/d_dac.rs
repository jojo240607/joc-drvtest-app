//! d_dac：12 位 DAC。判据：SET_VALUE 后 GET_VALUE 回读一致（DHR→DOR 链路）。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// dac0：SET_VALUE(0x800) → GET_VALUE 回读一致。
pub fn dac_ioctl(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("dac0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    let mut v: u16 = 0x0800;
    if dev.ioctl(DAC_IOCTL_SET_VALUE, (&mut v as *mut u16).cast()) != 0 {
        return Verdict::Fail("SET_VALUE 失败");
    }
    let mut g: u16 = 0;
    if dev.ioctl(DAC_IOCTL_GET_VALUE, (&mut g as *mut u16).cast()) != 0 {
        return Verdict::Fail("GET_VALUE 失败");
    }
    if g != v {
        return Verdict::Fail("DAC 值未回读一致");
    }

    Verdict::Pass("dac0 SET/GET_VALUE 往返一致")
}
