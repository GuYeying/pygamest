//! pygame.rapier —— rapier-starfish-debug 原生风格绑定面（2D v1，2026-10-24）。
//!
//! 接口风格定约：**第三方库等价物，不 pygame 化**——类型/方法名与
//! rapier-starfish-debug（rapier 0.36 词汇）一致，Rust 侧文档直接适用；
//! Python 难度控制 = builder 配置全走构造 kwargs（RP 链式返回 self 的
//! 成本 > 收益，v1 不做链式——原生 `RigidBodyBuilder::dynamic().translation(..)`
//! 的链式位在这里是 `RigidBodyBuilder("dynamic", translation=..)`）。
//!
//! 帧序定约（对齐 rapier-starfish-debug 设计文档 §6）：
//! `event 泵 → step_frame(dt) → collect → 游戏会话绘制 →
//! renderer.draw(sink)（叠加通道）→ flip`。**叠加绘制必须在 render 会话
//! 之后、flip 之前**（提交序 = 画家序；放会话内会被游戏 batch 覆盖）。
//!
//! 坐标契约：物理世界 = 米制、y 向上、原点在视口中心（ortho 相机，
//! `set_camera(ppm)` 调缩放）——与 pygame 像素坐标（y 向下、左上原点）
//! **不同系**，调试线独立成景显示。
//!
//! 惯例提醒（rapier 行为，非 bug）：默认 `sleep_color_multiplier` 把静止
//! 体画暗（亮度 ×0.2）——"物体不动就变暗消失"是 rapier 状态编码。
//!
//! 结构体声明 + `#[pyclass] impl` 块在 `mods/rapier.rs` 的 pymodule
//! 树内（RP 宏要求 struct 与方法收集块同宿；批次 43 拆分定约），
//! 本文件 = 会话实现本体（pyegui 同构：运行时 + draw_debug 排程）。

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use rapier_starfish_debug::dim2::debug::render::DebugLinesRenderer as SfDebugLinesRenderer;
use rapier_starfish_debug::dim2::debug::{collect as sf_collect, DebugDraw as SfDebugDraw,
    LineSink as SfLineSink};
use rapier_starfish_debug::dim2::prelude::{
    ChannelEventCollector, ColliderBuilder as SfColliderBuilder, ColliderHandle,
    CollisionEvent, ContactForceEvent, DebugRenderMode, QueryFilter, Ray as SfRay,
    IVector as SfIVector2, ActiveEvents as SfActiveEvents, Group as SfGroup, InteractionGroups,
    InteractionTestMode, RigidBodyBuilder as SfRigidBodyBuilder, RigidBodyHandle,
};
use rapier_starfish_debug::dim2::world::PhysicsWorld as SfPhysicsWorld;
use rustpython_derive::{FromArgs, pyclass};
use rustpython_vm::function::ArgIntoFloat;
use rustpython_vm::types::Constructor;
use rustpython_vm::{PyObjectRef, Py, PyRef, PyResult, VirtualMachine, builtins::PyType};

use crate::rpy::args::SfFPoint;
use super::{
    PyColliderBuilder, PyDebugDraw, PyDebugLinesRenderer, PyLineSink, PyPhysicsWorld,
    PyRigidBodyBuilder,
};

// ── handle 打包（(generation, index) → 不透明 u64；Python 侧当黑盒 int）──

fn pack_body(h: RigidBodyHandle) -> u64 {
    let (index, generation) = h.into_raw_parts();
    ((generation as u64) << 32) | index as u64
}

fn unpack_body(v: u64) -> RigidBodyHandle {
    RigidBodyHandle::from_raw_parts(v as u32, (v >> 32) as u32)
}

fn pack_collider(h: ColliderHandle) -> u64 {
    let (index, generation) = h.into_raw_parts();
    ((generation as u64) << 32) | index as u64
}

fn unpack_collider(v: u64) -> ColliderHandle {
    ColliderHandle::from_raw_parts(v as u32, (v >> 32) as u32)
}

// ── 收参结构体（构造 kwargs）────────────────────────────────────

/// PhysicsWorld(gravity=(0, -9.81), dt=1/60)
#[derive(FromArgs)]
pub(crate) struct PhysicsWorldArgs {
    /// 重力（米/秒²；y 向上为正）
    #[pyarg(any, optional)]
    pub(crate) gravity: Option<SfFPoint>,
    /// 固定步长（秒；缺省 1/60）
    #[pyarg(any, optional)]
    pub(crate) dt: Option<ArgIntoFloat>,
}

impl Constructor for PyPhysicsWorld {
    type Args = PhysicsWorldArgs;

    fn py_new(
        _cls: &Py<PyType>,
        args: Self::Args,
        _vm: &VirtualMachine,
    ) -> PyResult<Self> {
        let g = args.gravity.unwrap_or(SfFPoint(0.0, -9.81));
        let mut world = SfPhysicsWorld::new([g.0, g.1]);
        if let Some(dt) = args.dt {
            world.set_dt(dt.into_float() as f32);
        }
        // 碰撞事件通道（step_frame 恒经 collector 步进——触发力/撕裂事件
        // 的 sender 无接收端即静默丢弃）
        let (tx, rx) = std::sync::mpsc::channel();
        let (ftx, frx) = std::sync::mpsc::channel();
        let (_ttx, _trx) = std::sync::mpsc::channel();
        let collector = ChannelEventCollector::new(tx, ftx, _ttx);
        Ok(Self {
            inner: Rc::new(RefCell::new(world)),
            events_rx: RefCell::new(rx),
            force_rx: RefCell::new(frx),
            collector: RefCell::new(collector),
        })
    }
}

/// RigidBodyBuilder(kind="dynamic"|"fixed"|"kinematic", translation=,
///                  rotation=弧度, linvel=, angvel=, can_sleep=True,
///                  linear_damping=, angular_damping=, ccd_enabled=,
///                  lock_rotations=)
#[derive(FromArgs)]
pub(crate) struct RigidBodyBuilderArgs {
    #[pyarg(any)]
    pub(crate) kind: String,
    #[pyarg(any, optional)]
    pub(crate) translation: Option<SfFPoint>,
    /// 弧度（rapier 原生单位）
    #[pyarg(any, optional)]
    pub(crate) rotation: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) linvel: Option<SfFPoint>,
    #[pyarg(any, optional)]
    pub(crate) angvel: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) can_sleep: Option<bool>,
    /// 线性阻尼（缺省 0；数值越大停得越快）
    #[pyarg(any, optional)]
    pub(crate) linear_damping: Option<ArgIntoFloat>,
    /// 角阻尼
    #[pyarg(any, optional)]
    pub(crate) angular_damping: Option<ArgIntoFloat>,
    /// 连续碰撞检测（高速穿墙防线）
    #[pyarg(any, optional)]
    pub(crate) ccd_enabled: Option<bool>,
    /// 锁定旋转（不倒翁/角色胶囊）
    #[pyarg(any, optional)]
    pub(crate) lock_rotations: Option<bool>,
}

impl Constructor for PyRigidBodyBuilder {
    type Args = RigidBodyBuilderArgs;

    fn py_new(
        _cls: &Py<PyType>,
        args: Self::Args,
        vm: &VirtualMachine,
    ) -> PyResult<Self> {
        let mut b = match args.kind.as_str() {
            "dynamic" => SfRigidBodyBuilder::dynamic(),
            "fixed" => SfRigidBodyBuilder::fixed(),
            "kinematic" => SfRigidBodyBuilder::kinematic_velocity_based(),
            other => {
                return Err(vm.new_value_error(format!(
                    "kind 须为 'dynamic' / 'fixed' / 'kinematic'，实际 {other:?}"
                )))
            }
        };
        if let Some(t) = args.translation {
            b = b.translation([t.0, t.1].into());
        }
        if let Some(r) = args.rotation {
            b = b.rotation(r.into_float() as f32);
        }
        if let Some(v) = args.linvel {
            b = b.linvel([v.0, v.1].into());
        }
        if let Some(w) = args.angvel {
            b = b.angvel(w.into_float() as f32);
        }
        b = b.can_sleep(args.can_sleep.unwrap_or(true));
        if let Some(v) = args.linear_damping {
            b = b.linear_damping(v.into_float() as f32);
        }
        if let Some(v) = args.angular_damping {
            b = b.angular_damping(v.into_float() as f32);
        }
        if let Some(v) = args.ccd_enabled {
            b = b.ccd_enabled(v);
        }
        if args.lock_rotations.unwrap_or(false) {
            b = b.lock_rotations();
        }
        Ok(Self {
            builder: RefCell::new(Some(b)),
        })
    }
}

/// ColliderBuilder(kind="cuboid"|"ball"|"capsule"|"segment"|"triangle",
///                 半尺寸/半径/端点坐标, friction=, restitution=, density=,
///                 sensor=, translation=, events=,
///                 groups_memberships=+groups_filter=, force_events=,
///                 force_threshold=)
#[derive(FromArgs)]
pub(crate) struct ColliderBuilderArgs {
    /// "cuboid"（hx/hy 半尺寸）/ "ball"（radius）/ "capsule"（half_height
    /// + radius，纵轴 y）/ "segment"（ax ay bx by）/ "triangle"（六坐标）/
    /// "voxels"（voxel_size 方格边长 + voxels 扁平格子坐标）
    #[pyarg(any)]
    pub(crate) kind: String,
    /// 体素方格边长（米；"voxels" 用）
    #[pyarg(any, optional)]
    pub(crate) voxel_size: Option<ArgIntoFloat>,
    /// 体素格子坐标（扁平 [i,j, ...]，长度须为 2 的倍数；"voxels" 用）
    #[pyarg(any, optional)]
    pub(crate) voxels: Option<Vec<i32>>,
    #[pyarg(any, optional)]
    pub(crate) hx: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) hy: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) radius: Option<ArgIntoFloat>,
    /// capsule 半高（总长 = 2×half_height + 2×radius，纵轴 y）
    #[pyarg(any, optional)]
    pub(crate) half_height: Option<ArgIntoFloat>,
    /// segment / triangle 端点坐标
    #[pyarg(any, optional)]
    pub(crate) ax: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) ay: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) bx: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) by: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) x1: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) y1: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) x2: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) y2: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) x3: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) y3: Option<ArgIntoFloat>,
    /// 相对刚体原点的偏移（米）
    #[pyarg(any, optional)]
    pub(crate) translation: Option<SfFPoint>,
    #[pyarg(any, optional)]
    pub(crate) friction: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) restitution: Option<ArgIntoFloat>,
    #[pyarg(any, optional)]
    pub(crate) density: Option<ArgIntoFloat>,
    /// 传感器（只碰不撞，缺省 false）
    #[pyarg(any, optional)]
    pub(crate) sensor: Option<bool>,
    /// 产生碰撞事件（drain_events 只报开启者——触发器/碰撞检测入口，
    /// 缺省 false 省开销）
    #[pyarg(any, optional)]
    pub(crate) events: Option<bool>,
    /// 碰撞分组（两者同给生效）：memberships = 本体所属组位集，
    /// filter = 可碰撞组位集（bit i = 组 i；组 i 物体只与
    /// filter 含组 i 的物体碰撞——位掩码语义）
    #[pyarg(any, optional)]
    pub(crate) groups_memberships: Option<u32>,
    #[pyarg(any, optional)]
    pub(crate) groups_filter: Option<u32>,
    /// 冲击力事件（drain_force_events 产出——撞击力度门槛检测）
    #[pyarg(any, optional)]
    pub(crate) force_events: Option<bool>,
    /// 力事件门槛（总力幅值超过才发事件；缺省用 rapier 默认）
    #[pyarg(any, optional)]
    pub(crate) force_threshold: Option<ArgIntoFloat>,
}

impl Constructor for PyColliderBuilder {
    type Args = ColliderBuilderArgs;

    fn py_new(
        _cls: &Py<PyType>,
        args: Self::Args,
        vm: &VirtualMachine,
    ) -> PyResult<Self> {
        let mut c = match args.kind.as_str() {
            "cuboid" => {
                let (Some(hx), Some(hy)) = (args.hx, args.hy) else {
                    return Err(vm.new_value_error("cuboid 须给 hx / hy（半尺寸，米）"));
                };
                SfColliderBuilder::cuboid(hx.into_float() as f32, hy.into_float() as f32)
            }
            "ball" => {
                let Some(r) = args.radius else {
                    return Err(vm.new_value_error("ball 须给 radius（米）"));
                };
                SfColliderBuilder::ball(r.into_float() as f32)
            }
            "capsule" => {
                let (Some(hh), Some(r)) = (args.half_height, args.radius) else {
                    return Err(vm.new_value_error("capsule 须给 half_height / radius（米）"));
                };
                SfColliderBuilder::capsule_y(hh.into_float() as f32, r.into_float() as f32)
            }
            "segment" => {
                let (Some(ax), Some(ay), Some(bx), Some(by)) =
                    (args.ax, args.ay, args.bx, args.by)
                else {
                    return Err(vm.new_value_error("segment 须给 ax ay bx by（线段两端点）"));
                };
                SfColliderBuilder::segment(
                    [ax.into_float() as f32, ay.into_float() as f32].into(),
                    [bx.into_float() as f32, by.into_float() as f32].into(),
                )
            }
            "triangle" => {
                let (Some(x1), Some(y1), Some(x2), Some(y2), Some(x3), Some(y3)) = (
                    args.x1, args.y1, args.x2, args.y2, args.x3, args.y3,
                ) else {
                    return Err(vm.new_value_error("triangle 须给 x1 y1 x2 y2 x3 y3（三顶点）"));
                };
                SfColliderBuilder::triangle(
                    [x1.into_float() as f32, y1.into_float() as f32].into(),
                    [x2.into_float() as f32, y2.into_float() as f32].into(),
                    [x3.into_float() as f32, y3.into_float() as f32].into(),
                )
            }
            "voxels" => {
                let (Some(vs), Some(vox)) = (args.voxel_size, args.voxels.clone()) else {
                    return Err(
                        vm.new_value_error("voxels 须给 voxel_size（方格边长，米）/ voxels（扁平格子坐标）")
                    );
                };
                if vox.is_empty() || vox.len() % 2 != 0 {
                    return Err(vm.new_value_error("voxels 长度须为 2 的倍数（扁平 i,j）"));
                }
                let s = vs.into_float() as f32;
                let cells: Vec<_> = vox
                    .chunks_exact(2)
                    .map(|c| SfIVector2::new(c[0], c[1]))
                    .collect();
                SfColliderBuilder::voxels([s, s].into(), &cells)
            }
            other => {
                return Err(vm.new_value_error(format!(
                    "kind 须为 'cuboid' / 'ball' / 'capsule' / 'segment' / 'triangle' / 'voxels'，实际 {other:?}"
                )))
            }
        };
        if let Some(t) = args.translation {
            c = c.translation([t.0, t.1].into());
        }
        if let Some(f) = args.friction {
            c = c.friction(f.into_float() as f32);
        }
        if let Some(r) = args.restitution {
            c = c.restitution(r.into_float() as f32);
        }
        if let Some(d) = args.density {
            c = c.density(d.into_float() as f32);
        }
        if args.sensor.unwrap_or(false) {
            c = c.sensor(true);
        }
        // 碰撞分组（两 kwarg 同给生效——位掩码语义见字段注释）
        if let (Some(m), Some(f)) = (args.groups_memberships, args.groups_filter) {
            c = c.collision_groups(InteractionGroups::new(
                SfGroup::from_bits_truncate(m),
                SfGroup::from_bits_truncate(f),
                InteractionTestMode::And,
            ));
        }
        // 事件位集合并（碰撞事件 + 冲击力事件可同开）
        let mut ev = SfActiveEvents::default();
        if args.events.unwrap_or(false) {
            ev |= SfActiveEvents::COLLISION_EVENTS;
        }
        if args.force_events.unwrap_or(false) {
            ev |= SfActiveEvents::CONTACT_FORCE_EVENTS;
            if let Some(t) = args.force_threshold {
                c = c.contact_force_event_threshold(t.into_float() as f32);
            }
        }
        if ev != SfActiveEvents::default() {
            c = c.active_events(ev);
        }
        Ok(Self {
            builder: RefCell::new(Some(c)),
        })
    }
}

/// DebugDraw(collider_shapes=True, contacts=True, joints=False,
///           aabbs=False, axes=False)——渲染内容位集（rapier 原生语义）
#[derive(FromArgs)]
pub(crate) struct DebugDrawArgs {
    #[pyarg(any, optional)]
    pub(crate) collider_shapes: Option<bool>,
    #[pyarg(any, optional)]
    pub(crate) contacts: Option<bool>,
    #[pyarg(any, optional)]
    pub(crate) joints: Option<bool>,
    #[pyarg(any, optional)]
    pub(crate) aabbs: Option<bool>,
    #[pyarg(any, optional)]
    pub(crate) axes: Option<bool>,
}

fn mode_from_args(
    args: &DebugDrawArgs,
    base: DebugRenderMode,
) -> DebugRenderMode {
    let mut m = base;
    let set = |on: Option<bool>, flag: DebugRenderMode, m: &mut DebugRenderMode| match on {
        Some(true) => *m |= flag,
        Some(false) => *m &= !flag,
        None => {}
    };
    set(args.collider_shapes, DebugRenderMode::COLLIDER_SHAPES, &mut m);
    set(args.contacts, DebugRenderMode::CONTACTS, &mut m);
    set(args.joints, DebugRenderMode::JOINTS, &mut m);
    set(args.aabbs, DebugRenderMode::COLLIDER_AABBS, &mut m);
    set(args.axes, DebugRenderMode::RIGID_BODY_AXES, &mut m);
    m
}

impl Constructor for PyDebugDraw {
    type Args = DebugDrawArgs;

    fn py_new(
        _cls: &Py<PyType>,
        args: Self::Args,
        _vm: &VirtualMachine,
    ) -> PyResult<Self> {
        // 缺省 = 原生 DebugDraw::default() 的位集（COLLIDER_SHAPES | CONTACTS）
        let base = DebugRenderMode::COLLIDER_SHAPES | DebugRenderMode::CONTACTS;
        let mode = mode_from_args(&args, base);
        Ok(Self {
            inner: RefCell::new(SfDebugDraw::new(mode)),
        })
    }
}

/// DebugLinesRenderer(ppm=50.0)——调试线绘制器（GPU 惰性装配：首次
/// draw() 时须已 set_mode；表面尺寸变化自动重设相机）
#[derive(FromArgs)]
pub(crate) struct DebugLinesRendererArgs {
    /// 像素/米（ortho 相机缩放）
    #[pyarg(any, optional)]
    pub(crate) ppm: Option<ArgIntoFloat>,
}

impl Constructor for PyDebugLinesRenderer {
    type Args = DebugLinesRendererArgs;

    fn py_new(
        _cls: &Py<PyType>,
        args: Self::Args,
        _vm: &VirtualMachine,
    ) -> PyResult<Self> {
        Ok(Self {
            ppm: Cell::new(args.ppm.map(|p| p.into_float() as f32).unwrap_or(50.0)),
            follow_global: Cell::new(false),
            states: RefCell::new(Vec::new()),
        })
    }
}

// ── 渲染器惰性装配状态（GPU 句柄在首次 draw 时取——set_mode 后必有）──

pub(crate) struct RendererGpu {
    pub(crate) renderer: SfDebugLinesRenderer,
    pub(crate) access: Arc<starfish_render::render_resource_access::RenderResourceAccess>,
    pub(crate) viewport: (u32, u32),
}

// ── pymethod 实现（#[pyclass] impl 块——每类型 crate 内唯一）──────

// builder 无实例方法面——仅声明构造协议归属（RP 空属性 impl 块）
#[pyclass(with(Constructor))]
impl PyRigidBodyBuilder {}

#[pyclass(with(Constructor))]
impl PyColliderBuilder {}

#[pyclass(with(Constructor))]
impl PyPhysicsWorld {
    /// step_frame(real_dt) -> int：真实帧距累积积分（0 = 余量不足一步；
    /// 上限节流防死亡螺旋）
    #[pymethod]
    fn step_frame(&self, real_dt: ArgIntoFloat) -> u32 {
        self.inner
            .borrow_mut()
            .step_frame_with_events(real_dt.into_float() as f32, &*self.collector.borrow())
    }

    /// step()：固定步长积分一次
    #[pymethod]
    fn step(&self) {
        self.inner.borrow_mut().step();
    }

    /// set_dt(dt)：固定步长（秒）
    #[pymethod]
    fn set_dt(&self, dt: ArgIntoFloat) {
        self.inner.borrow_mut().set_dt(dt.into_float() as f32);
    }

    /// sim_time() -> float：已仿真时长（秒；与真实时间解耦）
    #[pymethod]
    fn sim_time(&self) -> f32 {
        self.inner.borrow().sim_time()
    }

    /// insert(body_builder, collider_builder) -> (body_handle, collider_handle)
    /// ——handle 是不透明 int（黑盒，回传给查询方法）；builder 一次性消费
    #[pymethod]
    fn insert(
        &self,
        body: PyRef<PyRigidBodyBuilder>,
        collider: PyRef<PyColliderBuilder>,
        vm: &VirtualMachine,
    ) -> PyResult<(u64, u64)> {
        let rb = body
            .builder
            .borrow_mut()
            .take()
            .ok_or_else(|| vm.new_value_error("RigidBodyBuilder 已被消费（insert 一次性，重建再插）"))?;
        let cb = collider
            .builder
            .borrow_mut()
            .take()
            .ok_or_else(|| vm.new_value_error("ColliderBuilder 已被消费（insert 一次性，重建再插）"))?;
        let (bh, ch) = self.inner.borrow_mut().get().insert(rb, cb);
        Ok((pack_body(bh), pack_collider(ch)))
    }

    /// translation(body_handle) -> (x, y)（米，世界系）
    #[pymethod]
    fn translation(&self, body_handle: u64, vm: &VirtualMachine) -> PyResult<(f32, f32)> {
        let w = self.inner.borrow();
        let body = w
            .get_ref()
            .bodies
            .get(unpack_body(body_handle))
            .ok_or_else(|| vm.new_value_error("无效的 body handle"))?;
        let t = body.translation();
        Ok((t.x, t.y))
    }

    /// rotation(body_handle) -> float（弧度）
    #[pymethod]
    fn rotation(&self, body_handle: u64, vm: &VirtualMachine) -> PyResult<f32> {
        let w = self.inner.borrow();
        let body = w
            .get_ref()
            .bodies
            .get(unpack_body(body_handle))
            .ok_or_else(|| vm.new_value_error("无效的 body handle"))?;
        Ok(body.rotation().angle())
    }

    /// linvel(body_handle) -> (vx, vy)（米/秒）
    #[pymethod]
    fn linvel(&self, body_handle: u64, vm: &VirtualMachine) -> PyResult<(f32, f32)> {
        let w = self.inner.borrow();
        let body = w
            .get_ref()
            .bodies
            .get(unpack_body(body_handle))
            .ok_or_else(|| vm.new_value_error("无效的 body handle"))?;
        let v = body.linvel();
        Ok((v.x, v.y))
    }

    /// is_sleeping(body_handle) -> bool（静止体休眠态）
    #[pymethod]
    fn is_sleeping(&self, body_handle: u64, vm: &VirtualMachine) -> PyResult<bool> {
        let w = self.inner.borrow();
        let body = w
            .get_ref()
            .bodies
            .get(unpack_body(body_handle))
            .ok_or_else(|| vm.new_value_error("无效的 body handle"))?;
        Ok(body.is_sleeping())
    }

    // ── 写入面（2D v2：游戏操控 / 触发器 / 查询）──────────────────

    /// set_translation(body_handle, (x, y))：瞬移（自动唤醒）
    #[pymethod]
    fn set_translation(&self, body_handle: u64, pos: SfFPoint, vm: &VirtualMachine) -> PyResult<()> {
        let mut w = self.inner.borrow_mut();
        let body = w
            .get()
            .bodies
            .get_mut(unpack_body(body_handle))
            .ok_or_else(|| vm.new_value_error("无效的 body handle"))?;
        body.set_translation([pos.0, pos.1].into(), true);
        Ok(())
    }

    /// set_rotation(body_handle, angle)：设姿态（弧度；自动唤醒）
    #[pymethod]
    fn set_rotation(&self, body_handle: u64, angle: ArgIntoFloat, vm: &VirtualMachine) -> PyResult<()> {
        let mut w = self.inner.borrow_mut();
        let body = w
            .get()
            .bodies
            .get_mut(unpack_body(body_handle))
            .ok_or_else(|| vm.new_value_error("无效的 body handle"))?;
        body.set_rotation(
            rapier_starfish_debug::dim2::prelude::Rotation::new(angle.into_float() as f32),
            true,
        );
        Ok(())
    }

    /// set_linvel(body_handle, (vx, vy))：设线速度（自动唤醒）
    #[pymethod]
    fn set_linvel(&self, body_handle: u64, v: SfFPoint, vm: &VirtualMachine) -> PyResult<()> {
        let mut w = self.inner.borrow_mut();
        let body = w
            .get()
            .bodies
            .get_mut(unpack_body(body_handle))
            .ok_or_else(|| vm.new_value_error("无效的 body handle"))?;
        body.set_linvel([v.0, v.1].into(), true);
        Ok(())
    }

    /// set_angvel(body_handle, w)：设角速度（自动唤醒）
    #[pymethod]
    fn set_angvel(&self, body_handle: u64, w: ArgIntoFloat, vm: &VirtualMachine) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        let body = world
            .get()
            .bodies
            .get_mut(unpack_body(body_handle))
            .ok_or_else(|| vm.new_value_error("无效的 body handle"))?;
        body.set_angvel(w.into_float() as f32, true);
        Ok(())
    }

    /// apply_impulse(body_handle, (ix, iy))：施加冲量（自动唤醒）
    #[pymethod]
    fn apply_impulse(&self, body_handle: u64, impulse: SfFPoint, vm: &VirtualMachine) -> PyResult<()> {
        let mut w = self.inner.borrow_mut();
        let body = w
            .get()
            .bodies
            .get_mut(unpack_body(body_handle))
            .ok_or_else(|| vm.new_value_error("无效的 body handle"))?;
        body.apply_impulse([impulse.0, impulse.1].into(), true);
        Ok(())
    }

    /// wake_up(body_handle)：唤醒休眠体
    #[pymethod]
    fn wake_up(&self, body_handle: u64, vm: &VirtualMachine) -> PyResult<()> {
        let mut w = self.inner.borrow_mut();
        w.get().wake_up(unpack_body(body_handle), false);
        Ok(())
    }

    /// set_gravity((gx, gy))：改重力（下一帧生效）
    #[pymethod]
    fn set_gravity(&self, g: SfFPoint) {
        self.inner.borrow_mut().get().gravity = [g.0, g.1].into();
    }

    /// remove_body(body_handle) -> bool：移除刚体（连带其全部碰撞体；
    /// 返回是否存在并已移除）
    #[pymethod]
    fn remove_body(&self, body_handle: u64, vm: &VirtualMachine) -> PyResult<bool> {
        let mut w = self.inner.borrow_mut();
        Ok(w.get()
            .remove_body_with_colliders(unpack_body(body_handle), true)
            .is_some())
    }

    /// drain_events() -> [(kind, collider_handle1, collider_handle2), ...]：
    /// 排空自上帧以来的碰撞事件（kind = "started" | "stopped"；仅
    /// `ColliderBuilder(events=True)` 的碰撞体产生事件——传感器触发器同
    /// 此通道）。每帧调用一次，不清也不丢（通道缓冲按步累积）。
    #[pymethod]
    fn drain_events(&self, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let mut out: Vec<PyObjectRef> = Vec::new();
        loop {
            let ev = match self.events_rx.borrow_mut().try_recv() {
                Ok(ev) => ev,
                Err(_) => break,
            };
            let (kind, h1, h2) = match ev {
                CollisionEvent::Started(h1, h2, _) => ("started", h1, h2),
                CollisionEvent::Stopped(h1, h2, _) => ("stopped", h1, h2),
                _ => continue,
            };
            let s: PyObjectRef = vm.ctx.new_str(kind).into();
            let a: PyObjectRef = vm.ctx.new_int(pack_collider(h1) as i64).into();
            let b: PyObjectRef = vm.ctx.new_int(pack_collider(h2) as i64).into();
            let tup: PyObjectRef = vm.new_tuple((s, a, b)).into();
            out.push(tup);
        }
        Ok(vm.ctx.new_list(out).into())
    }

    /// drain_force_events() -> [(h1, h2, 总力幅值, 最大接触力幅值)]：冲击
    /// 力事件排空（collider 须 force_events=True 开启 + force_threshold
    /// 设门槛——落体撞地伤害判定/击打反馈的判定源）。
    #[pymethod]
    fn drain_force_events(&self, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let mut out: Vec<PyObjectRef> = Vec::new();
        loop {
            let ev = match self.force_rx.borrow_mut().try_recv() {
                Ok(ev) => ev,
                Err(_) => break,
            };
            let tup: PyObjectRef = vm
                .new_tuple((
                    pack_collider(ev.collider1) as i64,
                    pack_collider(ev.collider2) as i64,
                    ev.total_force_magnitude as f64,
                    ev.max_force_magnitude as f64,
                ))
                .into();
            out.push(tup);
        }
        Ok(vm.ctx.new_list(out).into())
    }

    /// set_collider_groups(handle, memberships, filter)：运行时改碰撞
    /// 分组（句柄取自 drain_events/force_events 事件或 insert 产物；
    /// 换阵营/临时免伤的运行时开关）。
    #[pymethod]
    fn set_collider_groups(
        &self,
        handle: u64,
        memberships: u32,
        filter: u32,
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        let g = InteractionGroups::new(
            SfGroup::from_bits_truncate(memberships),
            SfGroup::from_bits_truncate(filter),
            InteractionTestMode::And,
        );
        match self.inner.borrow_mut().get().colliders.get_mut(unpack_collider(handle)) {
            Some(co) => {
                co.set_collision_groups(g);
                Ok(())
            }
            None => Err(vm.new_value_error("collider 句柄无效")),
        }
    }

    /// cast_ray(origin, dir, max_toi) -> None | (collider_handle, 距离)：
    /// 射线查询（dir 非单位向量按其模长定射线长；max_toi 为 dir 模长的
    /// 倍数上限，None = 缺省 100.0——RP 方法参数无缺省，Python 侧显式传
    /// None）。⚠️ 时序契约：insert 后**须至少 step 一次**方可查询
    /// （broad_phase 时序——实测 insert 后立即查询返回 None）。
    #[pymethod]
    fn cast_ray(
        &self,
        origin: SfFPoint,
        dir: SfFPoint,
        max_toi: Option<ArgIntoFloat>,
        vm: &VirtualMachine,
    ) -> PyResult<Option<(u64, f64)>> {
        let w = self.inner.borrow();
        let ray = SfRay::new([origin.0, origin.1].into(), [dir.0, dir.1].into());
        let max = max_toi.map(|m| m.into_float() as f32).unwrap_or(100.0);
        Ok(w.get_ref()
            .cast_ray(&ray, max, true, QueryFilter::default())
            .map(|(ch, toi)| (pack_collider(ch), toi as f64)))
    }

    /// intersection_pair(h1, h2) -> bool：两碰撞体当前是否相交
    #[pymethod]
    fn intersection_pair(&self, h1: u64, h2: u64, vm: &VirtualMachine) -> PyResult<bool> {
        let w = self.inner.borrow();
        Ok(w.get_ref()
            .intersection_pair(unpack_collider(h1), unpack_collider(h2))
            .is_some())
    }
}

#[pyclass(with(Constructor))]
impl PyDebugDraw {
    /// set_mode(collider_shapes=, contacts=, joints=, aabbs=, axes=)
    /// ——运行期改渲染内容位（只更新给出的位）
    #[pymethod]
    fn set_mode(&self, args: DebugDrawArgs) {
        let mut d = self.inner.borrow_mut();
        let base = d.pipeline().mode;
        d.pipeline().mode = mode_from_args(&args, base);
    }
}

#[pyclass(with(Constructor))]
impl PyLineSink {
    /// line_count() -> int：当前线段数（collect 后有效）
    #[pymethod]
    fn line_count(&self) -> u64 {
        self.inner.borrow().line_count() as u64
    }

    /// is_empty() -> bool
    #[pymethod]
    fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }

    /// clear()：清空缓冲（保 capacity）
    #[pymethod]
    fn clear(&self) {
        self.inner.borrow_mut().clear();
    }
}

impl Constructor for PyLineSink {
    type Args = ();

    fn py_new(
        _cls: &Py<PyType>,
        _args: Self::Args,
        _vm: &VirtualMachine,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: RefCell::new(SfLineSink::new()),
        })
    }
}

#[pyclass(with(Constructor))]
impl PyDebugLinesRenderer {
    /// set_camera(ppm)：ortho 缩放（像素/米；下次 draw_debug 生效——
    /// 全部样本数槽置脏）
    #[pymethod]
    fn set_camera(&self, ppm: ArgIntoFloat) {
        self.ppm.set(ppm.into_float() as f32);
        self.follow_global.set(false); // 显式相机优先（全局跟随解除）
        for (_, st) in self.states.borrow_mut().iter_mut() {
            st.viewport = (0, 0); // 置脏 → 下次 draw_debug 重设相机
        }
    }

    /// follow_global_camera(enable=True)：切全局相机跟随（渲染架构
    /// 批次 B）——draw 前同步引擎全局 MVP（`pygame.render.set_camera`
    /// 一处设置、全体生效）。注意 2D 调试线世界坐标为米：Auto2d 全局
    /// 相机按像素正交，米坐标直入会过小——跟随时请配 Manual 相机
    /// （set_camera 时按 ppm 计入视图矩阵）。再调 set_camera 即解除。
    #[pymethod]
    fn follow_global_camera(&self, enable: Option<bool>) {
        self.follow_global.set(enable.unwrap_or(true));
    }
}

// ── 模块级自由函数（mods/rapier.rs 的 #[pymodule] 面委托目标）──────

/// collect(world, draw, sink)：物理状态 → 线段流（每帧渲染前调用；
/// sink 内部先清后收，零分配复用）
pub(crate) fn collect(
    world: PyRef<PyPhysicsWorld>,
    draw: PyRef<PyDebugDraw>,
    sink: PyRef<PyLineSink>,
) {
    sf_collect(
        &world.inner.borrow(),
        &mut draw.inner.borrow_mut(),
        &mut sink.inner.borrow_mut(),
    );
}

/// draw_debug(renderer, sink)（`t.draw_debug(renderer, sink)` 的本体，
/// t = `with screen.render()` 的会话对象）：会话探针 → 泛化本体（样本数
/// = 屏 MSAA）。每帧一次；2D 通道无深度附件（线管线硬契约）。
pub(crate) fn draw_debug(
    renderer: PyRef<PyDebugLinesRenderer>,
    sink: PyRef<PyLineSink>,
    vm: &VirtualMachine,
) -> PyResult<()> {
    // 会话探针先行（同 draw_ui——会话外调用先报错）
    starfish_pygame::render::session::with_screen(|_| {}).map_err(|_| {
        vm.new_type_error("draw_debug 须在 `with screen.render():` 会话块内调用（t = 会话对象）")
    })?;
    let screen = starfish_pygame::display::get_screen();
    let samples = screen.sample_count();
    let size = starfish_pygame::render::DrawTarget::size(screen);
    let gpu = starfish_pygame::render::gpu_info().ok_or_else(|| {
        vm.new_runtime_error("GPU 未就绪：须在 pygame.display.set_mode 之后调用")
    })?;
    draw_debug_generic(
        renderer,
        sink,
        samples,
        size,
        // 全局跟随 MVP 与主绘制同一映射（present_mvp：逻辑画布域相机 +
        // letterbox 拟合统一组合——Auto 恒等、Manual 不再物理域双映射）
        screen.present_mvp().to_cols_array_2d(),
        false,
        &gpu,
        |boxed| {
            starfish_pygame::render::session::with_screen(|pass| {
                pass.defer_overlay(boxed);
            })
            .expect("会话探针刚确认存在（同栈同步，无中途收口路径）");
        },
    )
}

/// Surface 变体（`t.draw_debug(renderer, sink)`，t = `with surf.render()`
/// 会话对象）——排程到本面 flush 收口（画中画物理动画）。样本数 = 1
/// （离屏面无 MSAA），按目标分槽自持管线。
pub(crate) fn draw_debug_on_surface(
    renderer: PyRef<PyDebugLinesRenderer>,
    sink: PyRef<PyLineSink>,
    surface: &starfish_pygame::render::Surface,
    vm: &VirtualMachine,
) -> PyResult<()> {
    let gpu = starfish_pygame::render::gpu_info().ok_or_else(|| {
        vm.new_runtime_error("GPU 未就绪：须在 pygame.display.set_mode 之后调用")
    })?;
    draw_debug_generic(
        renderer,
        sink,
        1, // 离屏：无 MSAA
        starfish_pygame::render::DrawTarget::size(surface),
        // 离屏面无 letterbox（逻辑==物理）——全局相机按表面尺寸解算
        starfish_pygame::render::camera_mvp_for(
            starfish_pygame::render::DrawTarget::size(surface),
        )
        .to_cols_array_2d(),
        true, // 离屏目标：管线按 Rgba8Unorm 离屏 access 建
        &gpu,
        |boxed| surface.defer_overlay(boxed),
    )
}

/// draw_debug 本体（调度泛化，批次 48）：GPU 惰性装配（**按 (样本数,
/// 目标系) 分槽**——管线按 (color_format, sample_count) 分套是 wgpu
/// 硬约束：Screen = 交换链格式 access / Surface = Rgba8Unorm 离屏
/// access）+ 相机跟随（viewport 脏置）→ 经 schedule 排程（Screen =
/// present 收口 / Surface = flush 收口）。
fn draw_debug_generic(
    renderer: PyRef<PyDebugLinesRenderer>,
    sink: PyRef<PyLineSink>,
    samples: u32,
    size: (u32, u32),
    follow_mvp: [[f32; 4]; 4],
    offscreen: bool,
    gpu: &starfish_pygame::render::GpuInfo,
    schedule: impl FnOnce(Box<dyn FnOnce(&mut starfish_pygame::render::OverlayCtx) + 'static>),
) -> PyResult<()> {
    // 按目标系选 access（离屏 Rgba8Unorm / 主屏交换链格式——同 3D）
    let access = if offscreen {
        gpu.offscreen_access.clone()
    } else {
        gpu.access.clone()
    };
    {
        let mut slots = renderer.states.borrow_mut();
        let slot = match slots.iter_mut().find(|(k, _)| (k.0, k.1) == (samples, offscreen)) {
            Some(slot) => slot,
            None => {
                slots.push((
                    (samples, offscreen),
                    RendererGpu {
                        renderer: SfDebugLinesRenderer::new(&access, samples),
                        access,
                        viewport: (0, 0),
                    },
                ));
                slots.last_mut().expect("槽刚 push")
            }
        };
        let st = &mut slot.1;
        if renderer.follow_global.get() {
            // 全局相机跟随（批次 B）：每帧同步引擎全局 MVP（调用方按
            // 目标传入——Screen = present_mvp，与主绘制同一映射）
            st.renderer.set_camera_mvp(&st.access, follow_mvp);
            st.viewport = size;
        } else if st.viewport != size {
            st.renderer
                .set_camera_ortho(&st.access, size, renderer.ppm.get());
            st.viewport = size;
        }
    }
    // 排程（PyRef 'static——闭包持引用计数，present/flush 时段解引用载荷；
    // 主线程单线程契约下无并发借用）
    let key = (samples, offscreen);
    schedule(Box::new(move |octx| {
        let mut slots = renderer.states.borrow_mut();
        if let Some(slot) = slots.iter_mut().find(|(k, _)| *k == key) {
            let sink_ref = sink.inner.borrow();
            octx.with_pass("rapier_debug", false, |pass| {
                slot.1.renderer.draw(&slot.1.access, pass, &sink_ref);
            });
        }
    }));
    Ok(())
}
