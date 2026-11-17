//! pygame_physics —— 独立顶层 Python 模块（2026-10-24 架构抽象：自
//! （无后缀——`PhysicsWorld` 等），3D 类面随 feature "physics3d"
//! 与 rapier-starfish-debug dim2/dim3 镜像面一一对应。
//! `runtime_2d.rs` / `runtime_3d.rs` = 各维度会话实现本体；本文件 =
//! #[pymodule] 声明 + 类型。文件夹形态 = 新模块效仿模板。
//!
//! ⚠️ **上游限制（rapier 0.36.1 最新版未修，2026-11-06 实测）**：动态
//! 体与**体素复合体**（"voxels" / "voxelized_mesh"）接触后进入收敛
//! （接触流形收缩）即 panic `stale ContactRef manifold ordinal`
//! （manifold_store.rs:117——composite-pair 流形收缩竞态，单线程同样
//! 必现，非并行竞态）。规避：体素体与动态体轨迹**隔离**（测试脚本
//! 做法——静态独立摆放不受影响）；rapier 升级修复后解除。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use crate::rpy::{bridge, args};
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    builtins::{PyModule, PyStr, PyStrRef, PyType},
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// solid/fluid 子模块 + 总调度器（2026-11-17 分层重构——见 scheduler.rs 与
// solid/fluid 目录头。py API 零变化）
// 2026-11-17 solid/fluid 分层重构第一批；py API 零变化）
#[cfg(feature = "physics2d")]
pub(crate) mod solid;
#[cfg(feature = "fluid")]
pub(crate) mod fluid;
pub(crate) mod scheduler;


#[pymodule(name = "pygame_physics")]
    pub mod pygame_physics {
        use std::cell::{Cell, RefCell};
        use std::rc::Rc;
        #[cfg(feature = "physics2d")]
        use rapier_starfish_debug::dim2::debug::{DebugDraw as SfDebugDraw, LineSink as SfLineSink};
        #[cfg(feature = "physics2d")]
        use rapier_starfish_debug::dim2::prelude::{
            ColliderBuilder as SfColliderBuilder, RigidBodyBuilder as SfRigidBodyBuilder,
        };
        #[cfg(feature = "physics2d")]
        use rapier_starfish_debug::dim2::world::PhysicsWorld as SfPhysicsWorld;
        #[cfg(feature = "physics3d")]
        use rapier_starfish_debug::dim3::debug::{DebugDraw as SfDebugDraw3, LineSink as SfLineSink3};
        #[cfg(feature = "physics3d")]
        use rapier_starfish_debug::dim3::prelude::{
            ColliderBuilder as SfColliderBuilder3, RigidBodyBuilder as SfRigidBodyBuilder3,
        };
        #[cfg(feature = "physics3d")]
        use rapier_starfish_debug::dim3::world::PhysicsWorld as SfPhysicsWorld3;
        use crate::rpy::args::SfFPoint3;
        use rustpython_derive::{pyclass, FromArgs};
        use rustpython_vm::types::Constructor;
        use rustpython_vm::{
            PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
            builtins::{PyBool, PyStr, PyType},
        };

        // ── fluid 类面（feature "fluid"——salva 粒子流体；第二批扩展）──

        /// 流体世界（salva `FluidsPipeline` 薄壳——DFSPH 压力求解 +
        /// cuboid 采样流体/容器边界）。构造收 solid 世界引用做后续
        /// rapier 耦合（第二批：collider 边界化 + 力回传）。
        #[cfg(feature = "fluid")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "FluidWorld", unhashable = true)]
        pub(crate) struct PyFluidWorld {
            pub(crate) pipeline:
                RefCell<crate::rpy::pygame_physics::fluid::runtime_fluid::FluidsPipeline>,
            /// 粒子半径快照（add_fluid spacing 基准）
            pub(crate) particle_radius: Cell<f64>,
            /// 重力（step 时传入 salva；set_gravity 可改）
            pub(crate) gravity: RefCell<[f32; 3]>,
            pub(crate) next_fluid: Cell<u32>,
            pub(crate) next_boundary: Cell<u32>,
        }

        impl std::fmt::Debug for PyFluidWorld {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let n = self.pipeline.borrow().liquid_world.fluids().len();
                write!(f, "FluidWorld(<{} fluids>)", n)
            }
        }

        #[cfg(feature = "fluid")]
        #[derive(FromArgs)]
        pub(crate) struct FluidWorldArgs {
            /// 粒子半径（SPH 核半径 = radius × smoothing_factor × 2）
            #[pyarg(any, optional)]
            pub(crate) particle_radius: Option<f64>,
            /// 平滑系数（默认 2.0）
            #[pyarg(any, optional)]
            pub(crate) smoothing_factor: Option<f64>,
        }

        #[cfg(feature = "fluid")]
        impl Constructor for PyFluidWorld {
            type Args = FluidWorldArgs;

            fn py_new(
                _cls: &Py<PyType>,
                args: Self::Args,
                _vm: &VirtualMachine,
            ) -> PyResult<Self> {
                let radius = args.particle_radius.unwrap_or(0.2) as f32;
                let smoothing = args.smoothing_factor.unwrap_or(2.0) as f32;
                Ok(PyFluidWorld {
                    pipeline: RefCell::new(
                        crate::rpy::pygame_physics::fluid::runtime_fluid::make_pipeline(
                            radius, smoothing,
                        ),
                    ),
                    particle_radius: Cell::new(radius as f64),
                    gravity: RefCell::new([0.0, -9.81, 0.0]),
                    next_fluid: Cell::new(0),
                    next_boundary: Cell::new(0),
                })
            }
        }

        #[cfg(feature = "fluid")]
        #[pyclass(with(Constructor))]
        impl PyFluidWorld {
            /// set_gravity(x, y, z)：流体重力（默认 [0, -9.81, 0]）。
            #[pymethod]
            fn set_gravity(&self, x: f64, y: f64, z: f64) {
                *self.gravity.borrow_mut() = [x as f32, y as f32, z as f32];
            }

            /// add_fluid_cuboid(corner=(x,y,z), dims=(w,h,d), spacing=None)
            /// -> int：cuboid 均匀网格采样流体（spacing 缺省 = 2×粒子半径）。
            /// 返回 py 句柄（fluid_particles 查询用）。
            #[pymethod]
            fn add_fluid_cuboid(
                &self,
                args: FluidCuboidArgs,
                vm: &VirtualMachine,
            ) -> PyResult<u32> {
                let radius = self.particle_radius.get();
                let sp = args.spacing.unwrap_or(radius as f64 * 2.0).max(1e-4) as f32;
                let pts = crate::rpy::pygame_physics::fluid::runtime_fluid::sample_cuboid_grid(
                    [args.corner.0, args.corner.1, args.corner.2],
                    [args.dims.0, args.dims.1, args.dims.2],
                    sp,
                );
                if pts.is_empty() {
                    return Err(vm.new_value_error("dims too small - no sample points"));
                }
                let fluid = crate::rpy::pygame_physics::fluid::runtime_fluid::make_fluid(
                    pts, radius as f32,
                );
                let mut pw = self.pipeline.borrow_mut();
                pw.liquid_world.add_fluid(fluid);
                let py_handle = self.next_fluid.get();
                self.next_fluid.set(py_handle + 1);
                Ok(py_handle)
            }

            /// add_boundary_cuboid(corner, dims, spacing=None) -> int：
            /// 容器壁壳边界粒子（六面——静态零速；流体被约束在壳内）。
            #[pymethod]
            fn add_boundary_cuboid(
                &self,
                args: BoundaryCuboidArgs,
                vm: &VirtualMachine,
            ) -> PyResult<u32> {
                let radius = self.particle_radius.get();
                let sp = args.spacing.unwrap_or(radius as f64 * 2.0).max(1e-4) as f32;
                let pts = crate::rpy::pygame_physics::fluid::runtime_fluid::sample_shell(
                    [args.corner.0, args.corner.1, args.corner.2],
                    [args.dims.0, args.dims.1, args.dims.2],
                    sp,
                );
                let bw = crate::rpy::pygame_physics::fluid::runtime_fluid::make_boundary(pts);
                let mut pw = self.pipeline.borrow_mut();
                pw.liquid_world.add_boundary(bw);
                let py_handle = self.next_boundary.get();
                self.next_boundary.set(py_handle + 1);
                Ok(py_handle)
            }

            /// step(dt)：DFSPH 压力求解步进（调度器 fluid 关闭 = 直返）。
            #[pymethod]
            fn step(&self, dt: f64, vm: &VirtualMachine) -> PyResult<()> {
                if !crate::rpy::pygame_physics::scheduler::fluid_enabled() {
                    return Ok(());
                }
                let g = *self.gravity.borrow();
                let gravity = crate::rpy::pygame_physics::fluid::runtime_fluid::SVector::new(g[0] as f32, g[1] as f32, g[2] as f32);
                self.pipeline
                    .borrow_mut()
                    .liquid_world
                    .step(dt.max(1e-6) as f32, &gravity);
                Ok(())
            }

            /// fluid_particles(handle) -> (positions, velocities)：粒子
            /// 出口（平面化 [x,y,z,...] × 2 列表——instanced 球渲染供数）。
            #[pymethod]
            fn fluid_particles(
                &self,
                handle: u32,
                vm: &VirtualMachine,
            ) -> PyResult<(PyObjectRef, PyObjectRef)> {
                use crate::rpy::pygame_physics::fluid::runtime_fluid::SVector;
                let pw = self.pipeline.borrow();
                let fluid = pw
                    .liquid_world
                    .fluids()
                    .iter()
                    .nth(handle as usize)
                    .map(|(_, f)| f)
                    .ok_or_else(|| {
                        vm.new_value_error(format!("fluid handle {} not found", handle))
                    })?;
                let mut pos_out: Vec<PyObjectRef> =
                    Vec::with_capacity(fluid.positions.len() * 3);
                let mut vel_out: Vec<PyObjectRef> =
                    Vec::with_capacity(fluid.velocities.len() * 3);
                for p in &fluid.positions {
                    pos_out.push(vm.ctx.new_float(p.x as f64).into());
                    pos_out.push(vm.ctx.new_float(p.y as f64).into());
                    pos_out.push(vm.ctx.new_float(p.z as f64).into());
                }
                for v in &fluid.velocities {
                    vel_out.push(vm.ctx.new_float(v.x as f64).into());
                    vel_out.push(vm.ctx.new_float(v.y as f64).into());
                    vel_out.push(vm.ctx.new_float(v.z as f64).into());
                }
                let lv = vm.ctx.new_list(pos_out).into();
                let ln = vm.ctx.new_list(vel_out).into();
                Ok((lv, ln))
            }

            /// fluid_particle_count(handle) -> int：粒子数（instanced
            /// 渲染 instance 数基准）。
            #[pymethod]
            fn fluid_particle_count(&self, handle: u32, vm: &VirtualMachine) -> PyResult<u32> {
                let pw = self.pipeline.borrow();
                let n = pw
                    .liquid_world
                    .fluids()
                    .iter()
                    .nth(handle as usize)
                    .map(|(_, f)| f.positions.len())
                    .unwrap_or(0);
                Ok(n as u32)
            }

            /// particle_radius() -> float：粒子半径（渲染球尺寸基准）。
            #[pymethod]
            fn particle_radius(&self) -> f64 {
                self.particle_radius.get() as f64
            }

            /// boundary_count() -> int：边界对象数（诊断）。
            #[pymethod]
            fn boundary_count(&self) -> usize {
                self.pipeline.borrow().liquid_world.boundaries().len()
            }
        }

        /// add_fluid_cuboid 收参
        #[cfg(feature = "fluid")]
        #[derive(FromArgs)]
        pub(crate) struct FluidCuboidArgs {
            #[pyarg(any)]
            pub(crate) corner: SfFPoint3,
            #[pyarg(any)]
            pub(crate) dims: SfFPoint3,
            #[pyarg(named, optional)]
            pub(crate) spacing: Option<f64>,
        }

        /// add_boundary_cuboid 收参
        #[cfg(feature = "fluid")]
        #[derive(FromArgs)]
        pub(crate) struct BoundaryCuboidArgs {
            #[pyarg(any)]
            pub(crate) corner: SfFPoint3,
            #[pyarg(any)]
            pub(crate) dims: SfFPoint3,
            #[pyarg(named, optional)]
            pub(crate) spacing: Option<f64>,
        }

        // ── 2D 类面（feature "physics2d"；无后缀——既有脚本零改动）──

        /// 物理世界（rapier `PhysicsWorld` 薄壳 + 固定步长积分器；2D）
        #[cfg(feature = "physics2d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "PhysicsWorld", unhashable = true)]
        pub(crate) struct PyPhysicsWorld {
            pub(crate) inner: Rc<RefCell<SfPhysicsWorld>>,
            /// 碰撞事件通道（step_frame 恒经 collector 步进；drain_events 排空）
            pub(crate) events_rx:
                RefCell<std::sync::mpsc::Receiver<rapier_starfish_debug::dim2::prelude::CollisionEvent>>,
            /// 冲击力事件通道（collider 开 force_events 才有产出；drain_force_events 排空）
            pub(crate) force_rx: RefCell<
                std::sync::mpsc::Receiver<rapier_starfish_debug::dim2::prelude::ContactForceEvent>,
            >,
            pub(crate) collector:
                RefCell<rapier_starfish_debug::dim2::prelude::ChannelEventCollector>,
        }

        #[cfg(feature = "physics2d")]
        impl std::fmt::Debug for PyPhysicsWorld {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&format!("PhysicsWorld(<sim_time={}>)", self.inner.borrow().sim_time()))
            }
        }

        /// 刚体 builder（构造 kwargs 收参；insert 一次性消费；2D）
        #[cfg(feature = "physics2d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "RigidBodyBuilder", unhashable = true)]
        pub(crate) struct PyRigidBodyBuilder {
            pub(crate) builder: RefCell<Option<SfRigidBodyBuilder>>,
        }

        #[cfg(feature = "physics2d")]
        impl std::fmt::Debug for PyRigidBodyBuilder {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("RigidBodyBuilder(<kwargs-built>)")
            }
        }

        /// 碰撞体 builder（构造 kwargs 收参；insert 一次性消费；2D）
        #[cfg(feature = "physics2d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "ColliderBuilder", unhashable = true)]
        pub(crate) struct PyColliderBuilder {
            pub(crate) builder: RefCell<Option<SfColliderBuilder>>,
        }

        #[cfg(feature = "physics2d")]
        impl std::fmt::Debug for PyColliderBuilder {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("ColliderBuilder(<kwargs-built>)")
            }
        }

        /// 调试渲染控制面（内容位集 = rapier DebugRenderMode；2D）
        #[cfg(feature = "physics2d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "DebugDraw", unhashable = true)]
        pub(crate) struct PyDebugDraw {
            pub(crate) inner: RefCell<SfDebugDraw>,
        }

        #[cfg(feature = "physics2d")]
        impl std::fmt::Debug for PyDebugDraw {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("DebugDraw(<mode-bits>)")
            }
        }

        /// 线段收集器（collect 的落点；每帧复用零分配；2D）
        #[cfg(feature = "physics2d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "LineSink", unhashable = true)]
        pub(crate) struct PyLineSink {
            pub(crate) inner: RefCell<SfLineSink>,
        }

        #[cfg(feature = "physics2d")]
        impl std::fmt::Debug for PyLineSink {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&format!("LineSink(<{} lines>)", self.inner.borrow().line_count()))
            }
        }

        /// 调试线绘制器（2D：ortho 相机；GPU 惰性装配——首次 draw 需已
        /// set_mode）
        #[cfg(feature = "physics2d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "DebugLinesRenderer", unhashable = true)]
        pub(crate) struct PyDebugLinesRenderer {
            pub(crate) ppm: Cell<f32>,
            /// 全局相机跟随（渲染架构批次 B；默认 false——ppm 显式相机
            /// 优先；follow_global_camera() 开启后 draw 前同步全局 MVP）
            pub(crate) follow_global: Cell<bool>,
            /// 按 (样本数, 目标系) 分槽（Screen MSAA 交换链格式 / 离屏 1x
            /// Rgba8Unorm 各持管线——批次 48 画中画 + 附件格式分套定约：
            /// 同一渲染器可同时服务两类目标）
            pub(crate) states: RefCell<
                Vec<((u32, bool), crate::rpy::pygame_physics::solid::runtime_2d::RendererGpu)>,
            >,
        }

        #[cfg(feature = "physics2d")]
        impl std::fmt::Debug for PyDebugLinesRenderer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&format!("DebugLinesRenderer(<ppm={}>)", self.ppm.get()))
            }
        }

        /// scheduler_status() -> {"solid": bool, "fluid": bool}：子系统
        /// 运行时开关快照（总调度器——feature 决定编译期有没有，调度器
        /// 决定运行时推不推；默认按 feature 全开）。
        #[pyfunction]
        fn scheduler_status(vm: &VirtualMachine) -> PyObjectRef {
            let (solid_flag, fluid_flag) =
                crate::rpy::pygame_physics::scheduler::status();
            let solid = solid_flag
                && (cfg!(feature = "physics2d") || cfg!(feature = "physics3d"));
            let fluid = fluid_flag && cfg!(feature = "fluid");
            let d = vm.ctx.new_dict();
            let _ = d.set_item("solid", vm.ctx.new_bool(solid).into(), vm);
            let _ = d.set_item("fluid", vm.ctx.new_bool(fluid).into(), vm);
            d.into()
        }

        /// scheduler_enable(name, on)：运行时开关子系统（name ∈
        /// "solid"/"fluid"；on=False = step 直返暂停推进——世界对象
        /// 生命周期不变）。未编译的子系统恒报错。
        #[pyfunction]
        fn scheduler_enable(
            name: PyRef<PyStr>,
            on: bool,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            let name_s = name
                .to_str()
                .ok_or_else(|| vm.new_value_error("name 含非 UTF-8 字符"))?;
            if name_s == "solid"
                && !(cfg!(feature = "physics2d") || cfg!(feature = "physics3d"))
            {
                return Err(vm.new_runtime_error(
                    "solid 子系统未编译（需 physics2d/physics3d feature）",
                ));
            }
            if name_s == "fluid" && !cfg!(feature = "fluid") {
                return Err(vm.new_runtime_error(
                    "fluid 子系统未编译（需 fluid feature）",
                ));
            }
            crate::rpy::pygame_physics::scheduler::set_enabled(&name_s, on)
                .map_err(|e| vm.new_value_error(e))
        }

        /// collect(world, draw, sink)：物理状态 → 线段流（2D；渲染前每帧
        /// 调用）
        #[cfg(feature = "physics2d")]
        #[pyfunction]
        fn collect(
            world: PyRef<PyPhysicsWorld>,
            draw: PyRef<PyDebugDraw>,
            sink: PyRef<PyLineSink>,
            _vm: &VirtualMachine,
        ) -> PyResult<()> {
            crate::rpy::pygame_physics::solid::runtime_2d::collect(world, draw, sink);
            Ok(())
        }

        // ── 3D 类面（feature "physics3d"；`3D` 后缀与 2D 类共存一模块）──

        /// 物理世界（3D：米制，重力如 gravity=(0, -9.81, 0)）
        #[cfg(feature = "physics3d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "PhysicsWorld3D", unhashable = true)]
        pub(crate) struct PyPhysicsWorld3D {
            pub(crate) inner: Rc<RefCell<SfPhysicsWorld3>>,
            /// 碰撞事件通道（step_frame 恒经 collector 步进；drain_events 排空）
            pub(crate) events_rx:
                RefCell<std::sync::mpsc::Receiver<rapier_starfish_debug::dim3::prelude::CollisionEvent>>,
            /// 冲击力事件通道（collider 开 force_events 才有产出；drain_force_events 排空）
            pub(crate) force_rx: RefCell<
                std::sync::mpsc::Receiver<rapier_starfish_debug::dim3::prelude::ContactForceEvent>,
            >,
            pub(crate) collector:
                RefCell<rapier_starfish_debug::dim3::prelude::ChannelEventCollector>,
        }

        #[cfg(feature = "physics3d")]
        impl std::fmt::Debug for PyPhysicsWorld3D {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&format!("PhysicsWorld3D(<sim_time={}>)", self.inner.borrow().sim_time()))
            }
        }

        /// 刚体 builder（3D：rotation 收欧拉角向量 (rx, ry, rz)）
        #[cfg(feature = "physics3d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "RigidBodyBuilder3D", unhashable = true)]
        pub(crate) struct PyRigidBodyBuilder3D {
            pub(crate) builder: RefCell<Option<SfRigidBodyBuilder3>>,
        }

        #[cfg(feature = "physics3d")]
        impl std::fmt::Debug for PyRigidBodyBuilder3D {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("RigidBodyBuilder3D(<kwargs-built>)")
            }
        }

        /// 碰撞体 builder（3D：cuboid 收 hx/hy/hz）
        #[cfg(feature = "physics3d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "ColliderBuilder3D", unhashable = true)]
        pub(crate) struct PyColliderBuilder3D {
            pub(crate) builder: RefCell<Option<SfColliderBuilder3>>,
        }

        #[cfg(feature = "physics3d")]
        impl std::fmt::Debug for PyColliderBuilder3D {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("ColliderBuilder3D(<kwargs-built>)")
            }
        }

        /// 调试渲染控制面（3D）
        #[cfg(feature = "physics3d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "DebugDraw3D", unhashable = true)]
        pub(crate) struct PyDebugDraw3D {
            pub(crate) inner: RefCell<SfDebugDraw3>,
        }

        #[cfg(feature = "physics3d")]
        impl std::fmt::Debug for PyDebugDraw3D {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("DebugDraw3D(<mode-bits>)")
            }
        }

        /// 线段收集器（3D）
        #[cfg(feature = "physics3d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "LineSink3D", unhashable = true)]
        pub(crate) struct PyLineSink3D {
            pub(crate) inner: RefCell<SfLineSink3>,
        }

        #[cfg(feature = "physics3d")]
        impl std::fmt::Debug for PyLineSink3D {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&format!("LineSink3D(<{} lines>)", self.inner.borrow().line_count()))
            }
        }

        /// 调试线绘制器（3D：look-at 透视相机 + 深度通道；GPU 惰性装配）
        #[cfg(feature = "physics3d")]
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_physics", name = "DebugLinesRenderer3D", unhashable = true)]
        pub(crate) struct PyDebugLinesRenderer3D {
            pub(crate) camera: RefCell<crate::rpy::pygame_physics::solid::runtime_3d::Camera3>,
            pub(crate) camera_epoch: Cell<u32>,
            /// **显式 MVP 直写**（2026-11-17 线框对齐原语——与主绘制
            /// 自管相机共用同一矩阵的绝对对齐口；Some = 优先于 lookat/
            /// 全局跟随）
            pub(crate) explicit_mvp:
                RefCell<Option<crate::rpy::pygame::math::math::PyMat4>>,
            /// 全局相机跟随（渲染架构批次 B）：true = draw 前同步引擎
            /// 全局 MVP（camera_mvp_for——set_camera 一处设置全体生效）；
            /// 默认 false（构造的 look-at 相机 = 显式，优先于全局）
            pub(crate) follow_global: Cell<bool>,
            /// 按 (样本数, 目标系) 分槽（同 2D——批次 48 画中画 + 附件
            /// 格式分套定约）
            pub(crate) states: RefCell<
                Vec<((u32, bool), crate::rpy::pygame_physics::solid::runtime_3d::RendererGpu3D)>,
            >,
        }

        #[cfg(feature = "physics3d")]
        impl std::fmt::Debug for PyDebugLinesRenderer3D {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let c = self.camera.borrow();
                f.write_str(&format!(
                    "DebugLinesRenderer3D(<eye={:?} fov={:.3}>)",
                    c.eye, c.fov_y
                ))
            }
        }

        /// collect3d(world, draw, sink)：3D 物理状态 → 线段流（渲染前每帧
        /// 调用）
        #[cfg(feature = "physics3d")]
        #[pyfunction]
        fn collect3d(
            world: PyRef<PyPhysicsWorld3D>,
            draw: PyRef<PyDebugDraw3D>,
            sink: PyRef<PyLineSink3D>,
            _vm: &VirtualMachine,
        ) -> PyResult<()> {
            crate::rpy::pygame_physics::solid::runtime_3d::collect(world, draw, sink);
            Ok(())
        }
    }

// ── 文件级 re-export（外部引用归位：pygame 会话的 draw_debug* 钩子）──
#[cfg(feature = "physics2d")]
pub(crate) use pygame_physics::{PyPhysicsWorld, PyRigidBodyBuilder, PyColliderBuilder, PyDebugDraw, PyLineSink, PyDebugLinesRenderer};
#[cfg(any(feature = "physics2d", feature = "physics3d"))]
pub(crate) use pygame_physics::module_def;
#[cfg(feature = "physics3d")]
pub(crate) use pygame_physics::{PyPhysicsWorld3D, PyRigidBodyBuilder3D, PyColliderBuilder3D, PyDebugDraw3D, PyLineSink3D, PyDebugLinesRenderer3D};
