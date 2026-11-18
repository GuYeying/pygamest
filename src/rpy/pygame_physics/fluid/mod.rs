//! fluid 子模块——流体物理面（salva 0.11 SPH 粒子流体）。
//!
//! 2026-11-17 solid/fluid 分层重构第二批：`FluidWorld` = salva
//! `FluidsPipeline` 薄壳（DFSPH 压力求解器 + 粒子世界）。
//! 第一批面：cuboid 采样流体/容器边界 + step + 粒子位置读取（渲染
//! 供数）。rapier 刚体耦合（FluidsPipeline::step 的 ColliderSet/
//! RigidBodySet 通路）为第二批——coupling 输入经 solid world 逃生口
//! 可得（字段 pub）。
//!
//! 调度器：`pygame_physics.scheduler_status/enable` 运行时开关（关闭
//! = step 直返不推进——世界对象生命周期仍归脚本持有）。

#[cfg(feature = "fluid")]
pub(crate) mod runtime_fluid;

// fluid2d = salva2d 0.11（与 salva3d 同一 src 的 dim2 编译形态——API
// 同代同型，2D 耦合走 physics2d 刚体轴）
#[cfg(feature = "fluid2d")]
pub(crate) mod runtime_fluid2d;
