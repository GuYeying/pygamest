//! solid 子模块——刚体物理面（rapier）：2D/3D 双维度运行时。
//! 2026-11-17 solid/fluid 分层重构第一批：自顶层平移入目录（py API
//! 零变化），fluid（salva 流体）为平级子模块（另行挂载）。

#[cfg(feature = "physics2d")]
pub(crate) mod runtime_2d;
#[cfg(feature = "physics3d")]
pub(crate) mod runtime_3d;
