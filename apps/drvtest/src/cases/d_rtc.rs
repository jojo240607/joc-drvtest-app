//! d_rtc：实时时钟。ioctl 传 `rtc_time_t {hour,min,sec}` / `rtc_date_t {year,month,day,wday}`
//!（布局镜像 drv/rtc.h）。判据：GET 字段合法 + SET→GET 往返（秒允许 1-2s 走时偏移）。

use rtos_app_sdk::ioctl::*;

use crate::cap;
use crate::verdict::{Ctx, Verdict};

/// 镜像 drv/rtc.h rtc_time_t（3 字节）。
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RtcTime {
    hour: u8,
    min: u8,
    sec: u8,
}

/// 镜像 drv/rtc.h rtc_date_t（6 字节）。
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RtcDate {
    year: u16,
    month: u8,
    day: u8,
    wday: u8,
}

/// rtc0：时间 GET 合法 → SET{10:30:00} → GET 回读（秒偏移 ≤2）→ 日期 GET 合法。
pub fn rtc_time(_ctx: &mut Ctx) -> Verdict {
    let dev = match cap::open("rtc0") {
        Ok(d) => d,
        Err(r) => return Verdict::Skip(r),
    };

    // GET 时间字段合法。
    let mut t = RtcTime { hour: 0xFF, min: 0xFF, sec: 0xFF, ..Default::default() };
    if dev.ioctl(RTC_IOCTL_GET_TIME, (&mut t as *mut RtcTime).cast()) != 0 {
        return Verdict::Fail("GET_TIME 失败");
    }
    if t.hour > 23 || t.min > 59 || t.sec > 59 {
        return Verdict::Fail("时间字段越界");
    }

    // SET → GET 往返（RTC 持续走时，秒允许 0..2）。
    let mut set = RtcTime { hour: 10, min: 30, sec: 0 };
    if dev.ioctl(RTC_IOCTL_SET_TIME, (&mut set as *mut RtcTime).cast()) != 0 {
        return Verdict::Fail("SET_TIME 失败");
    }
    let mut g = RtcTime { hour: 0, min: 0, sec: 0 };
    if dev.ioctl(RTC_IOCTL_GET_TIME, (&mut g as *mut RtcTime).cast()) != 0 {
        return Verdict::Fail("GET_TIME#2 失败");
    }
    if g.hour != 10 || g.min != 30 {
        return Verdict::Fail("时间未写入（SET/GET 不一致）");
    }
    if g.sec > 2 {
        return Verdict::Fail("秒偏移过大（RTC 走时异常）");
    }

    // 日期 GET 合法（部分平台可能不支持 → Skip）。
    let mut d = RtcDate { year: 0, month: 0xFF, day: 0xFF, wday: 0 };
    if dev.ioctl(RTC_IOCTL_GET_DATE, (&mut d as *mut RtcDate).cast()) != 0 {
        return Verdict::Skip("GET_DATE 不可用（平台无日期支持）");
    }
    if d.year < 2000 || d.year > 2099 || d.month < 1 || d.month > 12 || d.day < 1 || d.day > 31 {
        return Verdict::Fail("日期字段越界");
    }

    Verdict::Pass("rtc0 时间 SET/GET 往返 + 日期读取正常")
}
