//! 总调度器——solid/fluid 子系统的运行时开关（2026-11-17 solid/fluid
//! 分层重构批次）。
//!
//! 语义定约：**feature 决定编译期有没有**（physics2d/physics3d/fluid
//! 能力轴），**调度器决定运行时推不推**（开关 = step 直返暂停，不销
//! 毁世界对象——生命周期仍归脚本持有）。默认按 feature 全开。
//!
//! 消费点：`PyPhysicsWorld.step_frame` / `PyFluidWorld.step` 入口各查
//! 一次本模块 flag——关闭即跳过推进（返回值语义不变：step_frame 回 0
//! 步、fluid step 直返 Ok）。

use std::cell::Cell;

thread_local! {
    static SOLID_ON: Cell<bool> = const { Cell::new(true) };
    static FLUID_ON: Cell<bool> = const { Cell::new(true) };
}

/// solid 子系统当前是否推进（step 入口消费）。
pub(crate) fn solid_enabled() -> bool {
    SOLID_ON.with(|s| s.get())
}

/// fluid 子系统当前是否推进（step 入口消费）。
pub(crate) fn fluid_enabled() -> bool {
    FLUID_ON.with(|s| s.get())
}

/// 运行时开关（py 面 scheduler_enable 的承载——name ∈ "solid"/"fluid"）。
pub(crate) fn set_enabled(name: &str, on: bool) -> Result<(), String> {
    match name {
        "solid" => SOLID_ON.with(|s| s.set(on)),
        "fluid" => FLUID_ON.with(|s| s.set(on)),
        other => return Err(format!(
            "子系统未识别：{other:?}（solid/fluid——按编译期 feature 决定可用面）"
        )),
    }
    Ok(())
}

/// 双子系统开关状态快照（py 面 scheduler_status 的承载；顺序 solid, fluid）。
pub(crate) fn status() -> (bool, bool) {
    (solid_enabled(), fluid_enabled())
}
