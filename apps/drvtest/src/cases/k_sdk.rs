//! SDK/RTOS 基础冒烟：任务拉起、tick 推进、msleep 精度、信号量/互斥量 IPC。
//! 这些不是外设驱动，而是驱动用例共同依赖的「地基」——先证明它们成立。

use core::ffi::c_void;

use rtos_app_sdk::rtos::{msleep, spawn, tick_count, Semaphore, Mutex, RTOS_PRIO_BH_MED};

use crate::cap;
use crate::stacks;
use crate::verdict::{Ctx, Verdict};

/* ---- 用例 1：tick_count 单调推进 + msleep 精度 ---- */

pub fn tick_and_msleep(_ctx: &mut Ctx) -> Verdict {
    let t0 = tick_count();
    msleep(10);
    let dt = tick_count().wrapping_sub(t0);
    if dt >= 8 && dt <= 12 {
        Verdict::Pass("tick 推进 10ms±2")
    } else {
        Verdict::Fail("msleep(10) 实际偏移")
    }
}

/* ---- 用例 2：spawn 拉起 worker 任务并周期给信号量 ---- */

static mut WORKER_SEM: Semaphore = Semaphore::uninit();

extern "C" fn worker_entry(_arg: *mut c_void) {
    loop {
        msleep(20);
        unsafe {
            WORKER_SEM.give();
        }
    }
}

pub fn spawn_and_sem(_ctx: &mut Ctx) -> Verdict {
    unsafe {
        WORKER_SEM.init();
        spawn(
            "drvtest_worker",
            worker_entry,
            RTOS_PRIO_BH_MED,
            stacks::WORKER_STACK.as_mut_ptr(),
            stacks::WORKER_STACK.len(),
        );
        // 第一次 wait 应立即返回（worker 首拍 20ms 后 give；预算 500ms 内必到）。
        let t0 = tick_count();
        WORKER_SEM.wait();
        let first = tick_count().wrapping_sub(t0);
        // 第二次 wait 需等 worker 下一拍（~20ms），证明 worker 持续存活。
        let t1 = tick_count();
        WORKER_SEM.wait();
        let second = tick_count().wrapping_sub(t1);
        if first <= 400 && second >= 10 && second <= 400 {
            Verdict::Pass("worker 任务存活 + sem 信号量 IPC 正常")
        } else {
            Verdict::Fail("worker/sem 时序异常")
        }
    }
}

/* ---- 用例 3：互斥量 lock/unlock 往返 ---- */

static mut TEST_MUTEX: Mutex = Mutex::uninit();

pub fn mutex_roundtrip(_ctx: &mut Ctx) -> Verdict {
    unsafe {
        TEST_MUTEX.init(0);
        TEST_MUTEX.lock();
        let a = tick_count();
        TEST_MUTEX.unlock();
        TEST_MUTEX.lock();
        let b = tick_count();
        TEST_MUTEX.unlock();
        if b.wrapping_sub(a) < 100 {
            Verdict::Pass("mutex lock/unlock 往返正常")
        } else {
            Verdict::Fail("mutex 往返异常耗时")
        }
    }
}

/* ---- 用例 4：dev_get 服务表可枚举（uart0 存在性） ---- */

pub fn slot_device_table(_ctx: &mut Ctx) -> Verdict {
    match cap::get("uart0") {
        Ok(d) => {
            if !d.name().is_empty() {
                Verdict::Pass("g_app_slot 设备服务表可用 (uart0 存在)")
            } else {
                Verdict::Fail("uart0 设备名为空")
            }
        }
        Err(_r) => Verdict::Fail("uart0 不可见"),
    }
}
