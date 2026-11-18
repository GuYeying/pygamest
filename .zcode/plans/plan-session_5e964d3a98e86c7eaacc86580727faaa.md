## Context

用户确认按上游 crate 结构分轴：crates.io 的 salva2d / salva3d 是两个独立 crate（同一 src、cfg 分维、API 同代同型），pygamest feature 轴对应分 **fluid2d / fluid3d**。rapier2d 0.36.1 已在 lock（零新增依赖版本）。特殊 feature（tray 等）全部保持 opt-in 不入 default（用户裁决）。

## 实施步骤

### 1. 依赖与 feature（pygamest Cargo.toml）
```toml
salva2d = { version = "0.11.0", optional = true, features = ["rapier", "dim2", "f32"] }
# fluid2d = 2D SPH（salva2d 0.11——与 salva3d 同一 src 的 dim2 编译形态）
fluid2d = ["dep:salva2d", "dep:nalgebra035"]
# fluid3d = 3D SPH（salva3d 0.11）；fluid = 旧名过渡别名（fluid3d 同义，逐步退役）
fluid3d = ["fluid"]
fluid = ["dep:salva3d", "dep:nalgebra035"]
```

### 2. runtime_fluid2d.rs（新文件，fluid/ 子模块，cfg fluid2d）
从 runtime_fluid.rs（3D 母本）派生，4 处机械差异：
- `SVector = nalgebra035::Vector2<f32>`；gravity `[f32; 2]`；`RapierVector::new(x, y)` 两参
- `sample_cuboid_grid` 降双循环（矩形填充）；`sample_shell` 降四边
- coupling 模块 import 换 `rapier_starfish_debug::dim2::prelude::{ColliderSet, RigidBodySet, Vector}`
- 全部 `salva3d::` → `salva2d::`（integrations::rapier::FluidsPipeline 4 参 step / ColliderSampling::DynamicContactSampling / object / solver::XSPHViscosity 同名可用）
- make_fluid 同挂 XSPHViscosity(2.0, 1.0) 粘稠默认

### 3. mod.rs 类面（cfg fluid2d）
`PyFluidWorld2D`（pyclass "FluidWorld2D"）——与 FluidWorld（3D，类名不动零破坏）全同型 API：构造(particle_radius, smoothing_factor) / add_fluid_cuboid(corner, dims, spacing) / add_boundary_cuboid / **step(dt, solid=None)**（OptionalArg<PyPhysicsWorld>——2D 耦合；PhysicsWorld 无后缀 = 2D 类）/ fluid_particles / fluid_particle_count / set_gravity(x, y) / particle_radius / boundary_count
- fluid/mod.rs：`#[cfg(feature = "fluid2d")] pub(crate) mod runtime_fluid2d;`

### 4. dev 仓透传（Cargo.toml）
`fluid2d = ["pygamest/fluid2d"]`、`fluid3d = ["pygamest/fluid3d"]`；default 不动 + 定则注释（tray/skip-taskbar/click-through/multi-window 全 opt-in——跨平台不足，用户裁决 tray 也没必要）

### 5. 案例 fluid2d_test.py（2D 流体 debug 渲染支持）
- 场景：PhysicsWorld（2D 刚体）地面 + 斜面 fixed + FluidWorld2D 水柱，`fw2.step(1/60, solid)` DC 耦合
- **2D 流体渲染**：instanced quad（WGSL 片元圆判水蓝填充）+ per-particle uniform + `game_camera_ortho` 正交相机（cam.mvp() 直喂）
- **2D solid 线框**（T 键）：`collect(solid, draw, sink)` + `DebugLinesRenderer(ppm=)` + `t.draw_debug`（rapier_test.py 现成形态）
- 断言：耦合注册 ≥2 / 300 帧未穿地 / 滑落交互 / 静置衰减

### 6. 验证 + 文档 + 提交
- `cargo check --features fluid2d`（pygamest）→ dev `cargo +nightly build --features fluid2d` → fluid2d_test 300 帧验收 → fluid_test（3D，`--features fluid` 旧名回归零破坏）→ `cargo check`（default 回归）
- 文档：pygame_physics.md FluidWorld2D 段 + fluid2d/fluid3d 轴说明 + 进度跟踪条目
- 两仓 commit + push

## 关键文件
- D:\Projects\Rust\pygame-rs\Cargo.toml（salva2d 依赖 + fluid2d/fluid3d 轴）
- D:\Projects\Rust\pygame-rs\src\rpy\pygame_physics\fluid\runtime_fluid2d.rs（新）
- D:\Projects\Rust\pygame-rs\src\rpy\pygame_physics\fluid\mod.rs、mod.rs（PyFluidWorld2D 类面）
- D:\Projects\Rust\pygame-rs-dev\Cargo.toml、examples\fluid2d_test.py（新）
- D:\Projects\Rust\pygame-rs\docs-zh\source\api\pygame_physics.md、reference\收尾目标进度跟踪.md