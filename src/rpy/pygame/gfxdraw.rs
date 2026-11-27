//! pygame.gfxdraw 绑定（2026-10-24 自 native_pygame.rs 拆分；body 原样搬运）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use crate::rpy::{bridge, args};
use crate::rpy::pygame::pygame::{PySurface, PySurfaceSession, PyScreen, PyScreenSession, PyColor};
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    function::{FuncArgs, PyComparisonValue},
    types::{Comparable, Constructor, PyComparisonOp, Representable},
    builtins::{PyModule, PyStr, PyStrRef, PyType},
};
use std::cell::Cell;
#[pymodule]
    pub mod gfxdraw {
        use crate::rpy::pygame::pygame::{PyScreenSession, PySurface, PySurfaceSession, PyTargetsSession};
        use crate::rpy::args::{SfColor, SfPoints, SfRect};
        use rustpython_derive::pyclass;
        use rustpython_vm::PyPayload;
        use rustpython_vm::convert::TryFromObject;
        use rustpython_vm::builtins::{PyStr, PyStrRef};
        use rustpython_vm::types::Constructor;
        use rustpython_derive::FromArgs;
        use crate::rpy::bridge;
        use starfish_pygame::render::{DrawState, DrawTarget};
        use starfish_pygame::render::batch::BatchTopology;
        use rustpython_vm::{
            PyObjectRef, PyRef, PyResult, VirtualMachine,
            function::{ArgIntoFloat, FuncArgs},
        };

    // ── 动画运行时渲染集成（阶段5：spine / live2d / ozz 数据流经会话
    // batch 上屏——渲染中心在 pygame；各适配库自持管线，底层类型走
    // pygamest）────────────────────────────────────────────────────

    /// mesh(f, surface, positions, uvs, indices, color=None)：任意带贴图
    /// 三角网格通用口——positions/uvs 扁平 f32（uv 纹理域直传），indices
    /// 三角索引按 3 展开；外部动画运行时数据流的消费插座。
    #[pyfunction]
    fn mesh(
        f: PyObjectRef,
        surface: Option<PyRef<PySurface>>,
        positions: Vec<f32>,
        uvs: Vec<f32>,
        indices: Vec<u16>,
        color: Option<SfColor>,
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        let rgba: [f32; 4] = match color {
            Some(c) => [
                c.0[0] as f32 / 255.0,
                c.0[1] as f32 / 255.0,
                c.0[2] as f32 / 255.0,
                c.0[3] as f32 / 255.0,
            ],
            None => [1.0, 1.0, 1.0, 1.0],
        };
        // 纯色路径 = 1×1 白 Surface（draw_mesh 签名必选纹理——回退后形态）
        let white = starfish_pygame::render::Surface::from_rgba8(
            (1, 1),
            &[255, 255, 255, 255],
        );
        let tex: &starfish_pygame::render::Surface = match surface.as_deref() {
            Some(s) => &s.surface,
            None => &white,
        };
        use starfish_pygame::render::DrawTarget;
        match route(&f, vm)? {
            Target::Surf(s) => {
                DrawTarget::draw_mesh(&*s.surface, tex, &positions, &uvs, &indices, rgba)
            }
            Target::Active => bridge::with_session(vm, |t| {
                DrawTarget::draw_mesh(t, tex, &positions, &uvs, &indices, rgba)
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                DrawTarget::draw_mesh(t, tex, &positions, &uvs, &indices, rgba)
            })?,
        }
        Ok(())
    }

    /// draw_spine(f, skeleton, pages)：spine 骨架渲染——chine render()
    /// 命令流 → mesh 上屏（uv 翻 v：page 域 y 向下 → 引擎 v 向上；
    /// positions 为 spine 世界坐标——Auto2d 相机域直入）。pages[cmd.page]
    /// = 图集页 Surface。
    #[cfg(feature = "spine")]
    #[pyfunction]
    fn draw_spine(
        f: PyObjectRef,
        skeleton: PyRef<crate::rpy::pygame_spine::pygame_spine::PySpineSkeleton>,
        pages: Vec<PyRef<PySurface>>,
        transform: rustpython_vm::function::OptionalArg<PyRef<crate::rpy::pygame::math::math::PyMat4>>,
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        use starfish_pygame::render::DrawTarget;
        let sk = skeleton.inner.borrow();
        let cmds = pygame_spine::render::render(&sk);
        // 定位变换 CPU 烘（None = 恒等——spine 世界坐标仿射 xy）
        let mat = match transform {
            rustpython_vm::function::OptionalArg::Present(m) => Some(m.0.get()),
            rustpython_vm::function::OptionalArg::Missing => None,
        };
        fn draw_cmds<T: DrawTarget + ?Sized>(
            t: &T,
            cmds: &[pygame_spine::RenderCommand],
            pages: &[std::sync::Arc<starfish_pygame::render::Surface>],
            mat: Option<glam::Mat4>,
        ) {
            for c in cmds {
                let Some(page) = pages.get(c.page) else {
                    continue; // 图集页越界：跳过（数据面校验在 Python 侧）
                };
                let mut positions: Vec<f32> =
                    c.positions.iter().flat_map(|p| [p.x, p.y]).collect();
                if let Some(m) = mat {
                    for p in positions.chunks_exact_mut(2) {
                        let v = m * glam::Vec4::new(p[0], p[1], 0.0, 1.0);
                        p[0] = v.x;
                        p[1] = v.y;
                    }
                }
                // chine uv = PNG 原生 top-down 域（v=0 图顶）——与引擎纹理
                // 上传方向一致，直传零翻转
                let uvs: Vec<f32> = c.uvs.clone();
                let rgba = [c.color.r, c.color.g, c.color.b, c.color.a];
                t.draw_mesh(page, &positions, &uvs, &c.triangles, rgba);
            }
        }
        let page_surfaces: Vec<std::sync::Arc<starfish_pygame::render::Surface>> =
            pages.iter().map(|p| p.surface.clone()).collect();
        match route(&f, vm)? {
            Target::Surf(s) => {
                draw_cmds(&*s.surface, &cmds, &page_surfaces, mat);
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                draw_cmds(t, &cmds, &page_surfaces, mat);
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                draw_cmds(t, &cmds, &page_surfaces, mat);
                Ok(())
            })?,
        }
    }

    /// draw_ozz_quads(f, player, scale=1.0, cx=0.0, cy=0.0, size=14.0)：
    /// ozz 关节盒体动画渲染——每骨一个 quad（世界位 + x/y 基方向定向，
    /// 单次 draw_mesh 纯色路径 None 纹理批量上屏——67 关节 = 单 draw）。
    #[cfg(feature = "ozz")]
    #[derive(FromArgs)]
    struct OzzQuadsArgs {
        player: PyRef<crate::rpy::pygame_ozz::pygame_ozz::PyOzzPlayer>,
        #[pyarg(any, optional)]
        scale: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        cx: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        cy: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        size: Option<ArgIntoFloat>,
        /// 定位变换（None = 恒等——CPU 烘进 quad 顶点）
        #[pyarg(any, optional)]
        transform: Option<PyRef<crate::rpy::pygame::math::math::PyMat4>>,
    }

    #[cfg(feature = "ozz")]
    #[pyfunction]
    fn draw_ozz_quads(f: PyObjectRef, args: OzzQuadsArgs, vm: &VirtualMachine) -> PyResult<()> {
        use starfish_pygame::render::{BatchTopology, DrawTarget};
        let s = args.scale.map(|v| v.into_float() as f32).unwrap_or(1.0);
        let ox = args.cx.map(|v| v.into_float() as f32).unwrap_or(0.0);
        let oy = args.cy.map(|v| v.into_float() as f32).unwrap_or(0.0);
        // 盒体宽 = 屏域单位（与骨骼缩放解耦——骨骼域随资产变化）
        let hw = args
            .size
            .map(|v| v.into_float() as f32)
            .unwrap_or(14.0)
            * 0.5;
        let pos = args.player.inner.borrow().joint_positions_xy();
        let parents = args.player.inner.borrow().joint_parents();
        // 盒体沿"父→关节"骨段方向定向（关节自身基向量随动画旋转剧烈，
        // 骨段方向才是可读的肢体朝向）；根关节画小方块。
        let mut positions: Vec<f32> = Vec::with_capacity(pos.len() * 8);
        let mut uvs: Vec<f32> = Vec::with_capacity(pos.len() * 8);
        let mut indices: Vec<u16> = Vec::with_capacity(pos.len() * 6);
        let pt = |i: usize| -> (f32, f32) {
            let (x, y) = pos[i];
            (x * s + ox, y * s + oy)
        };
        for (i, parent) in parents.iter().enumerate() {
            let (jx, jy) = pt(i);
            let corners: [(f32, f32); 4] = if *parent >= 0 {
                let Some(ppos) = pos.get(*parent as usize) else {
                    continue;
                };
                let (px_, py_) = pt(*parent as usize);
                let (dx, dy) = (jx - px_, jy - py_);
                let len = (dx * dx + dy * dy).sqrt().max(1e-6);
                let (dx, dy) = (dx / len, dy / len);
                let (per_x, per_y) = (-dy, dx);
                [
                    (jx + per_x * hw, jy + per_y * hw),
                    (jx - per_x * hw, jy - per_y * hw),
                    (px_ - per_x * hw, py_ - per_y * hw),
                    (px_ + per_x * hw, py_ + per_y * hw),
                ]
            } else {
                [
                    (jx - hw, jy - hw),
                    (jx + hw, jy - hw),
                    (jx + hw, jy + hw),
                    (jx - hw, jy + hw),
                ]
            };
            for (cx0, cy0) in corners {
                positions.push(cx0);
                positions.push(cy0);
            }
            uvs.extend_from_slice(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]);
            let b = i as u16 * 4;
            indices.extend_from_slice(&[b, b + 1, b + 2, b + 2, b + 3, b]);
        }
        fn draw_quads<T: DrawTarget + ?Sized>(
            t: &T,
            positions: &[f32],
            uvs: &[f32],
            indices: &[u16],
            white: &starfish_pygame::render::Surface,
        ) {
            t.draw_mesh(white, positions, uvs, indices, [1.0, 0.85, 0.45, 0.92]);
        }
        let white_q = starfish_pygame::render::Surface::from_rgba8((1, 1), &[255, 255, 255, 255]);
        // 定位变换 CPU 烘（生成后统一——仿射 xy，z=0）
        if let Some(m) = &args.transform {
            let mat = m.0.get();
            for p in positions.chunks_exact_mut(2) {
                let v = mat * glam::Vec4::new(p[0], p[1], 0.0, 1.0);
                p[0] = v.x;
                p[1] = v.y;
            }
        }
        match route(&f, vm)? {
            Target::Surf(s2) => {
                draw_quads(&*s2.surface, &positions, &uvs, &indices, &white_q);
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                draw_quads(t, &positions, &uvs, &indices, &white_q);
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                draw_quads(t, &positions, &uvs, &indices, &white_q);
                Ok(())
            })?,
        }
    }

    /// draw_ozz_bones(f, player, scale=1.0, cx=0.0, cy=0.0)：ozz 骨架骨骼
    /// 线渲染（父子世界位 xy × scale + 中心偏移——Auto2d 中心系直入；
    /// ozz y 向上与引擎同构零翻转。蒙皮 mesh 面落地前的骨骼可视化形态）。
    #[cfg(feature = "ozz")]
    #[derive(FromArgs)]
    struct OzzBonesArgs {
        player: PyRef<crate::rpy::pygame_ozz::pygame_ozz::PyOzzPlayer>,
        #[pyarg(any, optional)]
        scale: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        cx: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        cy: Option<ArgIntoFloat>,
        /// 定位变换（None = 恒等——CPU 烘进线段坐标）
        #[pyarg(any, optional)]
        transform: Option<PyRef<crate::rpy::pygame::math::math::PyMat4>>,
    }

    #[cfg(feature = "ozz")]
    #[pyfunction]
    fn draw_ozz_bones(f: PyObjectRef, args: OzzBonesArgs, vm: &VirtualMachine) -> PyResult<()> {
        let (player, scale, cx, cy) = (args.player, args.scale, args.cx, args.cy);
        use starfish_pygame::render::BatchTopology;
        let s = scale.map(|v| v.into_float() as f32).unwrap_or(1.0);
        let ox = cx.map(|v| v.into_float() as f32).unwrap_or(0.0);
        let oy = cy.map(|v| v.into_float() as f32).unwrap_or(0.0);
        let pos = player.inner.borrow().joint_positions_xy();
        let parents = player.inner.borrow().joint_parents();
        fn draw_bones<T: starfish_pygame::render::DrawTarget + ?Sized>(
            t: &T,
            pos: &[(f32, f32)],
            parents: &[i16],
            s: f32,
            ox: f32,
            oy: f32,
            mat: Option<glam::Mat4>,
        ) {
            let px = |x: f32| (x * s + ox) as i32;
            let py = |y: f32| (y * s + oy) as i32;
            for (i, parent) in parents.iter().enumerate() {
                if *parent < 0 {
                    continue; // 根关节无父
                }
                let Some(ppos) = pos.get(*parent as usize) else {
                    continue;
                };
                let (jx, jy) = pos[i];
                // 定位变换 CPU 烘（i32 化前 f32 域——取整抖动消除）
                let seg = [
                    (px(jx) as f32, py(jy) as f32),
                    (px(ppos.0) as f32, py(ppos.1) as f32),
                ];
                let seg = match mat {
                    Some(m) => seg.map(|(x, y)| {
                        let v = m * glam::Vec4::new(x, y, 0.0, 1.0);
                        (v.x, v.y)
                    }),
                    None => seg,
                };
                t.push_points(
                    BatchTopology::Lines,
                    None,
                    &seg,
                    [1.0, 0.9, 0.3, 1.0],
                );
            }
        }
        match route(&f, vm)? {
            Target::Surf(s2) => {
                draw_bones(&*s2.surface, &pos, &parents, s, ox, oy, args.transform.as_deref().map(|m| m.0.get()));
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                draw_bones(t, &pos, &parents, s, ox, oy, args.transform.as_deref().map(|m| m.0.get()));
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                draw_bones(t, &pos, &parents, s, ox, oy, args.transform.as_deref().map(|m| m.0.get()));
                Ok(())
            })?,
        }
    }


        /// gfxdraw 目标：SurfaceSession 直绘（自有 batch）；Screen 会话
        /// （PyScreenSession）= 当前活跃会话（with_session 路由，须在 with 内）。
        pub(crate) enum Target<'a> {
            Surf(&'a PySurfaceSession),
            Active,
            /// MRT 活跃槽（with_mrt_session——镜像双写会话）
            Targets,
        }


        pub(crate) fn route<'a>(f: &'a PyObjectRef, vm: &VirtualMachine) -> PyResult<Target<'a>> {
            if let Some(ss) = f.downcast_ref::<PySurfaceSession>() {
                return Ok(Target::Surf(ss));
            }
            if f.downcast_ref::<PyScreenSession>().is_some() {
                return Ok(Target::Active);
            }
            if f.downcast_ref::<crate::rpy::pygame::pygame::PyTargetsSession>().is_some() {
                return Ok(Target::Targets);
            }
            Err(vm.new_type_error(
                "gfxdraw 目标应为渲染会话（with ...render() as f 的 f）",
            ))
        }

        // ── 渲染统一 draw 面：状态机口（相机/变换随 gfx 管线分域；
        // renderpass 作首参——与 SDL_gfx 签名同构，render 会话保持纯通道）──

        /// set_camera(f, camera)：会话相机切换（后续 gfx 绘制进新段；
        /// 仅本会话生效）。**域分层 v3**：收 PyGameCamera（唯一相机
        /// 类型——PyCamera 已退役；构造 logical==physical 时 fit 恒等
        /// = 纯投影，会话段数学不变）
        #[pyfunction]
        fn set_camera(
            f: PyObjectRef,
            camera: PyRef<crate::rpy::pygame::render::render::PyGameCamera>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => {
                    s.surface
                        .with_batch(|b| b.set_mvp(Some(camera.0.get().mvp())))
                }
                Target::Active => {
                    let cam = camera.0.get();
                    bridge::with_session(vm, |t| t.set_camera(&cam))?
                }
                Target::Targets => {
                    let cam = camera.0.get();
                    bridge::with_mrt_session(vm, |t| t.set_camera(&cam))?
                }
            }
            Ok(())
        }

        /// clear_camera(f)：清除会话相机覆盖（回按目标尺寸正交——
        /// 2026-11-12 改名：隐式兜底已退役，"clear" 才是实义）
        #[pyfunction]
        fn clear_camera(f: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.with_batch(|b| b.reset_mvp()),
                Target::Active => bridge::with_session(vm, |t| t.clear_camera())?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.clear_camera())?,
            }
            Ok(())
        }

        /// set_transform(f, model)：会话持久变换——**服务 draw_surface 系
        /// / draw_spine / draw_ozz 桥**（这些桥的顶点按此矩阵 CPU 烘）。
        /// draw_batch 不读此状态（transform 走 kwargs 参数自包含）。
        /// ⚠️ 退役条件：spine/ozz 桥 transform kwarg 化 + draw_surface
        /// 系迁移完成后删除（spine/ozz 桥 CPU 烘改造属下一批）。
        #[pyfunction]
        fn set_transform(
            f: PyObjectRef,
            model: PyRef<crate::rpy::pygame::math::math::PyMat4>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.with_batch(|b| b.set_model(model.0.get())),
                Target::Active => bridge::with_session(vm, |t| t.set_transform(model.0.get()))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.set_transform(model.0.get()))?,
            }
            Ok(())
        }


        /// reset_transform(f)：回恒等变换
        #[pyfunction]
        fn reset_transform(f: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.with_batch(|b| b.reset_model()),
                Target::Active => bridge::with_session(vm, |t| t.reset_transform())?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.reset_transform())?,
            }
            Ok(())
        }

        // ── push 统一口（v1 批次：VertexBuf + set_topology + push +
        // gen_* 生成器族）────────────────────────────────────────────
        // 数据模型：VertexBuf = 顶点位置流（f32 [x,y,z × N]）+ 可选索引
        // ——颜色/uv/变换全部 push 时按状态烘焙（与 push_geometry 同款，
        // Geometry 颜色通道本就是死数据）。生成器 = Rust 内生成位置流
        // （原点中心局部坐标——位置由 set_transform 赋予），数据全程
        // 不落地 Python 逐顶点结构。push 流 = sticky 语义（color/
        // topology/transform 跨 push 持续）；draw 函数族保持 one-shot
        // （pygame 兼容语义）不变。

        /// VertexBuf：顶点位置流容器（Rust Vec 持有——py 侧零逐顶点
        /// tuple 摆渡）。三条填充通道：add_point 单点 / add_points 扁平
        /// 数组 / extend bytes 直拷；gen_* 生成器产出同类型。
        ///
        /// **拓扑随数据走**（2026-11-25 定约——替代全局 set_topology
        /// sticky 状态）：线族生成器烤入 Lines、填充族 Triangles；py 可
        /// 经 `buf.topology` 属性覆写（"triangles"/"lines"）控制自己的
        /// 顶点数据解释——push 按缓冲自带拓扑分仓，零 sticky 陷阱。
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame.gfxdraw", name = "VertexBuf", unhashable = true)]
        pub(crate) struct PyVertexBuf {
            /// 扁平位置流 [x,y,z × N]
            pub(crate) points: std::cell::RefCell<Vec<f32>>,
            /// 可选索引（空 = 按拓扑逐 3/逐 2 解释）
            pub(crate) indices: std::cell::RefCell<Vec<u16>>,
            /// push 拓扑（None = Triangles；线族生成器烤入 Lines）
            pub(crate) topology: std::cell::RefCell<Option<starfish_pygame::render::batch::BatchTopology>>,
        }

        impl std::fmt::Debug for PyVertexBuf {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("VertexBuf(<pos-stream>)")
            }
        }

        /// py 构造：VertexBuf() 空缓冲（gen_* 生成器产出的同类型）
        impl rustpython_vm::types::Constructor for PyVertexBuf {
            type Args = FuncArgs;

            fn py_new(
                _cls: &rustpython_vm::Py<rustpython_vm::builtins::PyType>,
                _args: Self::Args,
                _vm: &VirtualMachine,
            ) -> PyResult<Self> {
                Ok(Self {
                    points: std::cell::RefCell::new(Vec::new()),
                    indices: std::cell::RefCell::new(Vec::new()),
                    topology: std::cell::RefCell::new(None),
                })
            }
        }

        #[pyclass(with(Constructor))]
        impl PyVertexBuf {
            /// topology 属性（"triangles" / "lines"——开发者覆写口；
            /// gen_* 已按语义烤入，覆写用于自定义流的解释控制）
            #[pygetset]
            fn topology(&self) -> String {
                match self
                    .topology
                    .borrow()
                    .unwrap_or(starfish_pygame::render::batch::BatchTopology::Triangles)
                {
                    starfish_pygame::render::batch::BatchTopology::Lines => "lines".to_owned(),
                    _ => "triangles".to_owned(),
                }
            }

            #[pygetset(setter)]
            fn set_topology(&self, value: PyStrRef, vm: &VirtualMachine) -> PyResult<()> {
                let s = value
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("topology 含非 UTF-8 字符"))?;
                let topo = match s {
                    "triangles" => starfish_pygame::render::batch::BatchTopology::Triangles,
                    "lines" => starfish_pygame::render::batch::BatchTopology::Lines,
                    other => {
                        return Err(vm.new_value_error(format!(
                            "topology 未识别：{other:?}（triangles/lines）"
                        )))
                    }
                };
                *self.topology.borrow_mut() = Some(topo);
                Ok(())
            }

            /// add_point(x, y)：追加 2D 点（z=0）。
            #[pymethod]
            fn add_point(&self, x: f64, y: f64) {
                self.points
                    .borrow_mut()
                    .extend([x as f32, y as f32, 0.0f32]);
            }

            /// add_point3(x, y, z)：追加 3D 点。
            #[pymethod]
            fn add_point3(&self, x: f64, y: f64, z: f64) {
                self.points
                    .borrow_mut()
                    .extend([x as f32, y as f32, z as f32]);
            }

            /// add_points(flat)：扁平一维数组批量追加
            /// （[x1,y1,z1, x2,y2,z2, ...]——一次 py 调用，Rust 内循环
            /// 解包，无中间 tuple；int/float 混合收）。长度须为 3 的倍数。
            #[pymethod]
            fn add_points(&self, flat: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
                let vals: Vec<f32> = vm
                    .extract_elements_with(&flat, |o| {
                        rustpython_vm::function::ArgIntoFloat::try_from_object(vm, o.clone())
                            .map(|f| f.into_float() as f32)
                    })
                    .map_err(|_| vm.new_type_error("add_points 项须为数字"))?;
                if vals.len() % 3 != 0 {
                    return Err(vm.new_value_error(format!(
                        "add_points 长度须为 3 的倍数（[x,y,z × N]），实际 {}",
                        vals.len()
                    )));
                }
                self.points.borrow_mut().extend(vals);
                Ok(())
            }

            /// extend(data)：bytes 直拷（f32 LE 内存块——struct.pack /
            /// array / numpy 产出，零解包零转换）。长度须为 12 的倍数
            /// （每点 3×4B）。
            #[pymethod]
            fn extend(&self, data: PyRef<rustpython_vm::builtins::PyBytes>, vm: &VirtualMachine) -> PyResult<()> {
                let bytes = data.as_bytes();
                if bytes.len() % 12 != 0 {
                    return Err(vm.new_value_error(format!(
                        "extend 长度须为 12 的倍数（每点 3×f32），实际 {}",
                        bytes.len()
                    )));
                }
                let mut out = self.points.borrow_mut();
                for chunk in bytes.chunks_exact(4) {
                    out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                }
                Ok(())
            }

            /// set_indices(indices)：设置索引（空 = 按拓扑逐组解释）。
            #[pymethod]
            fn set_indices(&self, indices: Vec<u16>) {
                *self.indices.borrow_mut() = indices;
            }

            /// clear()：清空位置流与索引（跨帧复用容器）。
            #[pymethod]
            fn clear(&self) {
                self.points.borrow_mut().clear();
                self.indices.borrow_mut().clear();
            }
        }

        /// Geometry → VertexBuf（生成器共用：抽 pos 流 + 索引；拓扑默认
        /// Triangles——线族 gen 后自行覆写 Lines）
        fn geo_to_buf(g: starfish_gfx::geometry::Geometry) -> PyVertexBuf {
            let points: Vec<f32> = g
                .vertices
                .iter()
                .flat_map(|v| v.pos.iter().copied())
                .collect();
            PyVertexBuf {
                points: std::cell::RefCell::new(points),
                indices: std::cell::RefCell::new(g.indices),
                topology: std::cell::RefCell::new(None),
            }
        }

        /// 生成器 kwargs 助手（segments 等可选 int）
        fn kw_arg_int(
            args: &FuncArgs,
            vm: &VirtualMachine,
            key: &str,
            default: u32,
        ) -> PyResult<u32> {
            use rustpython_vm::convert::TryFromObject;
            match args.kwargs.get(key) {
                Some(o) => u32::try_from_object(vm, o.clone()),
                None => Ok(default),
            }
        }

        // ── set_topology 全局口已移除（2026-11-25 定约——拓扑随数据走：
        //    gen_* 烤入 VertexBuf.topology（线族 Lines/填充族 Triangles），
        //    开发者经 buf.topology 属性覆写。sticky 全局状态曾致漏复位
        //    = 后续填充图元被按线段解释的错乱陷阱）──

        /// push 参数（data = VertexBuf | bytes | 扁平 list | [(x,y),...]；
        /// indices 可选——覆盖 VertexBuf 自带索引）
        #[derive(FromArgs)]
        struct PushArgs {
            f: PyObjectRef,
            data: PyObjectRef,
            #[pyarg(named, optional)]
            indices: Option<Vec<u16>>,
        }


        // ── push 口已删（2026-11-26 用户定夺——one-shot 统一定约：
        //    transform/color/wireframe 本次是本次、下次是下次各自独立，
        //    draw_batch 参数自包含即全部语义；旧 push 的 sticky 会话
        //    读取路径随之移除）──


        /// 顶点流 → 目标 batch 的**统一渲染核心**（2026-11-27 定约：
        /// draw_batch 与 3D 便捷口共用同一条渲染路径——几何/拓扑/索引
        /// 由调用方备好，本函数只管三路由 + 状态落段）。
        /// transform=None = 沿用会话当前变换（draw_batch 语义）；
        /// 3D 便捷口坐标已绝对化，显式传单位阵（不继承会话变换）。
        fn draw_stream_core(
            vm: &VirtualMachine,
            f: &PyObjectRef,
            topo: BatchTopology,
            points: &[f32],
            indices: &[u16],
            transform: Option<glam::Mat4>,
            color: Option<[f32; 4]>,
            state: DrawState,
            wireframe: bool,
        ) -> PyResult<()> {
            match route(f, vm)? {
                Target::Surf(s) => {
                    let c = color.unwrap_or_else(|| s.surface.current_draw_color());
                    s.surface.with_batch(|b| {
                        if let Some(m) = transform {
                            b.set_model(m);
                        }
                        b.push_stream_state(topo, points, indices, c, state, wireframe);
                    });
                }
                Target::Active => bridge::with_session(vm, |t| {
                    let c = color.unwrap_or_else(|| t.current_draw_color());
                    t.with_batch(|b| {
                        if let Some(m) = transform {
                            b.set_model(m);
                        }
                        b.push_stream_state(topo, points, indices, c, state, wireframe);
                    });
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    let c = color.unwrap_or_else(|| t.current_draw_color());
                    t.with_batch(|b| {
                        if let Some(m) = transform {
                            b.set_model(m);
                        }
                        b.push_stream_state(topo, points, indices, c, state, wireframe);
                    });
                })?,
            }
            Ok(())
        }


        /// SfColor(0..255) → 着色器色(0..1)
        fn sf_color(c: &SfColor) -> [f32; 4] {
            [
                c.0[0] as f32 / 255.0,
                c.0[1] as f32 / 255.0,
                c.0[2] as f32 / 255.0,
                c.0[3] as f32 / 255.0,
            ]
        }


        /// draw_batch 参数（**强类型 VertexBuf**——裸数据先经 VertexBuf
        /// 构造（add_points/extend/set_indices），数据通道归一；
        /// None = 沿用会话当前值）
        #[derive(FromArgs)]
        struct DrawBatchArgs {
            /// 渲染会话（with ...render() as f 的 f）
            f: PyObjectRef,
            /// 顶点批（拓扑/索引随数据）
            data: PyRef<PyVertexBuf>,
            /// 模型变换（None = 沿用会话当前变换）
            #[pyarg(any, optional)]
            transform: Option<PyRef<crate::rpy::pygame::math::math::PyMat4>>,
            /// 绘制色（None = 沿用会话当前色）
            #[pyarg(any, optional)]
            color: Option<SfColor>,
            /// 线框模式（None = 否；仅本批）
            #[pyarg(any, optional)]
            wireframe: Option<bool>,
            /// 混合档位（None = alpha；alpha/blend、noblend/opaque/none、
            /// premultiplied、additive、multiply）
            #[pyarg(any, optional)]
            blend: Option<String>,
            /// 剔除（None = none；none / back）
            #[pyarg(any, optional)]
            cull: Option<String>,
        }

        /// draw_batch(f, data, transform=None, color=None, wireframe=None,
        /// blend=None, cull=None)：**顶点批绘制统一口**——拓扑随数据走
        /// （VertexBuf 烤入 / buf.topology 覆写），变换/色/线框/混合/
        /// 剔除参数自包含（本次调用独立，不落回会话状态）。
        #[pyfunction]
        fn draw_batch(args: DrawBatchArgs, vm: &VirtualMachine) -> PyResult<()> {
            let state = DrawState::from_names(args.blend.as_deref(), args.cull.as_deref())
                .map_err(|e| vm.new_value_error(e))?;
            let wireframe = args.wireframe.unwrap_or(false);
            let topo = args
                .data
                .topology
                .borrow()
                .unwrap_or(BatchTopology::Triangles);
            let points = args.data.points.borrow().clone();
            let indices = args.data.indices.borrow().clone();
            let transform = args.transform.as_ref().map(|m| m.0.get());
            let color = args.color.as_ref().map(sf_color);
            // 唯一渲染路径：与 3D 便捷口共用 draw_stream_core
            draw_stream_core(
                vm,
                &args.f,
                topo,
                &points,
                &indices,
                transform,
                color,
                state,
                wireframe,
            )
        }

        /// set_blend_mode(**已移除**)：改用 draw_batch/draw_surface 的
        /// `blend=` 参数（alpha / noblend / premultiplied / additive / multiply）
        #[pyfunction]
        fn set_blend_mode(_mode: String, vm: &VirtualMachine) -> PyResult<()> {
            Err(vm.new_runtime_error(
                "set_blend_mode 已移除：请改用 draw_batch/draw_surface 的 blend= 参数（alpha / noblend / premultiplied / additive / multiply）",
            ))
        }

        /// set_cull_mode(**已移除**)：改用 draw_batch/draw_surface 的
        /// `cull=` 参数（none / back）
        #[pyfunction]
        fn set_cull_mode(_mode: String, vm: &VirtualMachine) -> PyResult<()> {
            Err(vm.new_runtime_error(
                "set_cull_mode 已移除：请改用 draw_batch/draw_surface 的 cull= 参数（none / back）",
            ))
        }

        // ── gen_* 生成器族（Rust 内生成位置流——原点中心局部坐标；
        // 位置由 draw_batch 的 transform= 赋予，性能 = 语义函数族同级）──

        /// gen_circle(radius, segments=48) -> VertexBuf：填充圆（扇形
        /// 三角化，原点中心）。
        #[pyfunction]
        fn gen_circle(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let radius = args
                .args
                .first()
                .and_then(|o| ArgIntoFloat::try_from_object(vm, o.clone()).ok())
                .map(|f| f.into_float() as f32)
                .ok_or_else(|| vm.new_type_error("gen_circle(radius, segments=48)"))?;
            let seg = kw_arg_int(&args, vm, "segments", 48)?;
            let g = starfish_gfx::geometry::shape2d::circle(
                glam::Vec2::ZERO,
                radius,
                seg,
                [1.0; 4],
            );
            Ok(geo_to_buf(g))
        }

        /// gen_circle_outline(radius, segments=64) -> VertexBuf：圆周线
        ///（1px，lines 拓扑消费，原点中心）。
        #[pyfunction]
        fn gen_circle_outline(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let radius = args
                .args
                .first()
                .and_then(|o| ArgIntoFloat::try_from_object(vm, o.clone()).ok())
                .map(|f| f.into_float() as f32)
                .ok_or_else(|| vm.new_type_error("gen_circle_outline(radius, segments=64)"))?;
            let seg = kw_arg_int(&args, vm, "segments", 64)?;
            let g = starfish_gfx::geometry::shape2d::circle_outline(
                glam::Vec2::ZERO,
                radius,
                seg,
                [1.0; 4],
            );
            let buf = geo_to_buf(g);
            buf.topology.replace(Some(starfish_pygame::render::batch::BatchTopology::Lines));
            Ok(buf)
        }

        /// gen_rect(w, h) -> VertexBuf：填充矩形（中心原点局部坐标）。
        #[pyfunction]
        fn gen_rect(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let dims = args
                .args
                .iter()
                .map(|o| ArgIntoFloat::try_from_object(vm, o.clone()).map(|f| f.into_float() as f32))
                .collect::<Result<Vec<f32>, _>>()
                .map_err(|_| vm.new_type_error("gen_rect(w, h)"))?;
            if dims.len() < 2 {
                return Err(vm.new_type_error("gen_rect(w, h)"));
            }
            let hw = dims[0] * 0.5;
            let hh = dims[1] * 0.5;
            let g = starfish_gfx::geometry::shape2d::rect(
                glam::vec2(-hw, -hh),
                glam::vec2(hw, hh),
                [1.0; 4],
            );
            Ok(geo_to_buf(g))
        }

        /// gen_rect_outline(w, h) -> VertexBuf：矩形周界线（中心原点，
        /// lines 拓扑消费）。
        #[pyfunction]
        fn gen_rect_outline(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let dims = args
                .args
                .iter()
                .map(|o| ArgIntoFloat::try_from_object(vm, o.clone()).map(|f| f.into_float() as f32))
                .collect::<Result<Vec<f32>, _>>()
                .map_err(|_| vm.new_type_error("gen_rect_outline(w, h)"))?;
            if dims.len() < 2 {
                return Err(vm.new_type_error("gen_rect_outline(w, h)"));
            }
            let hw = dims[0] * 0.5;
            let hh = dims[1] * 0.5;
            let g = starfish_gfx::geometry::shape2d::rect_outline(
                glam::vec2(-hw, -hh),
                glam::vec2(hw, hh),
                [1.0; 4],
            );
            let buf = geo_to_buf(g);
            buf.topology.replace(Some(starfish_pygame::render::batch::BatchTopology::Lines));
            Ok(buf)
        }

        /// gen_polygon(points) -> VertexBuf：多边形（扇化三角化；输入
        /// 点即局部坐标，按给定位置入流）。
        #[pyfunction]
        fn gen_polygon(points: SfPoints, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            if points.0.len() < 3 {
                return Err(vm.new_value_error("gen_polygon 至少 3 个点"));
            }
            let pts: Vec<glam::Vec2> = points
                .0
                .iter()
                .map(|(x, y)| glam::vec2(*x as f32, *y as f32))
                .collect();
            let g = starfish_gfx::geometry::shape2d::polygon(&pts, [1.0; 4]);
            Ok(geo_to_buf(g))
        }

        /// gen_cube(size) -> VertexBuf：立方体（中心原点，6 面三角化；
        /// 透视段相机消费，lines/triangles 拓扑均可用）。
        #[pyfunction]
        fn gen_cube(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let size = args
                .args
                .first()
                .and_then(|o| ArgIntoFloat::try_from_object(vm, o.clone()).ok())
                .map(|f| f.into_float() as f32)
                .ok_or_else(|| vm.new_type_error("gen_cube(size)"))?;
            let h = size * 0.5;
            let g = starfish_gfx::geometry::shape3d::cube(
                glam::vec3(-h, -h, -h),
                glam::vec3(h, h, h),
                [1.0; 4],
            );
            Ok(geo_to_buf(g))
        }

        /// gen_line(x1, y1, x2, y2) -> VertexBuf：线段（lines 拓扑消费；
        /// 输入点即局部坐标）。
        #[pyfunction]
        fn gen_line(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let p: Vec<f32> = args
                .args
                .iter()
                .map(|o| {
                    ArgIntoFloat::try_from_object(vm, o.clone()).map(|f| f.into_float() as f32)
                })
                .collect::<Result<Vec<f32>, _>>()
                .map_err(|_| vm.new_type_error("gen_line(x1, y1, x2, y2)"))?;
            if p.len() < 4 {
                return Err(vm.new_type_error("gen_line(x1, y1, x2, y2)"));
            }
            let g = starfish_gfx::geometry::shape2d::line(
                glam::vec2(p[0], p[1]),
                glam::vec2(p[2], p[3]),
                [1.0; 4],
            );
            let buf = geo_to_buf(g);
            buf.topology.replace(Some(starfish_pygame::render::batch::BatchTopology::Lines));
            Ok(buf)
        }

        /// gen_arc(radius, start_deg, stop_deg, segments=0) -> VertexBuf：
        /// 圆弧线（中心原点，度数制起止角，0° = +x 方向逆时针；lines
        /// 拓扑消费）。segments=0 = 按密度自适应（每 ~10° 一段）。
        /// 剖分本体归位 starfish-gfx shape2d::arc（2026-11-24 下沉定约
        /// ——本层只留 py 收参 + 密度策略）。
        #[pyfunction]
        fn gen_arc(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let p: Vec<f32> = args
                .args
                .iter()
                .map(|o| {
                    ArgIntoFloat::try_from_object(vm, o.clone()).map(|f| f.into_float() as f32)
                })
                .collect::<Result<Vec<f32>, _>>()
                .map_err(|_| vm.new_type_error("gen_arc(radius, start_deg, stop_deg, segments=0)"))?;
            if p.len() < 3 {
                return Err(vm.new_type_error("gen_arc(radius, start_deg, stop_deg, segments=0)"));
            }
            let (r, a0, a1) = (p[0], p[1], p[2]);
            // 分段密度策略（py API 默认语义）：每 ~10° 一段，clamp 4..256
            let sweep = (a1 - a0).abs();
            let seg = kw_arg_int(&args, vm, "segments", 0)?
                .max(((sweep / 10.0).ceil() as u32).clamp(4, 256));
            let g = starfish_gfx::geometry::shape2d::arc(
                glam::Vec2::ZERO,
                r,
                a0,
                a1,
                seg,
                [1.0; 4],
            );
            let buf = geo_to_buf(g);
            buf.topology.replace(Some(starfish_pygame::render::batch::BatchTopology::Lines));
            Ok(buf)
        }

        /// gen_ellipse(rx, ry, segments=48) -> VertexBuf：填充椭圆（扇形
        /// 三角化，原点中心）。
        #[pyfunction]
        fn gen_ellipse(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let p: Vec<f32> = args
                .args
                .iter()
                .map(|o| {
                    ArgIntoFloat::try_from_object(vm, o.clone()).map(|f| f.into_float() as f32)
                })
                .collect::<Result<Vec<f32>, _>>()
                .map_err(|_| vm.new_type_error("gen_ellipse(rx, ry, segments=48)"))?;
            if p.len() < 2 {
                return Err(vm.new_type_error("gen_ellipse(rx, ry, segments=48)"));
            }
            let seg = kw_arg_int(&args, vm, "segments", 48)?;
            let g = starfish_gfx::geometry::shape2d::ellipse(
                glam::Vec2::ZERO,
                glam::vec2(p[0], p[1]),
                seg,
                [1.0; 4],
            );
            Ok(geo_to_buf(g))
        }

        /// gen_polygon_outline(points) -> VertexBuf：多边形周界线（闭合
        /// 折线，lines 拓扑消费；输入点即局部坐标）。
        #[pyfunction]
        fn gen_polygon_outline(points: SfPoints, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            if points.0.len() < 3 {
                return Err(vm.new_value_error("gen_polygon_outline 至少 3 个点"));
            }
            let pts: Vec<glam::Vec2> = points
                .0
                .iter()
                .map(|(x, y)| glam::vec2(*x as f32, *y as f32))
                .collect();
            let g = starfish_gfx::geometry::shape2d::polyline(&pts, true, [1.0; 4]);
            let buf = geo_to_buf(g);
            buf.topology.replace(Some(starfish_pygame::render::batch::BatchTopology::Lines));
            Ok(buf)
        }

        /// gen_sphere(radius, segments_u=16, segments_v=12) -> VertexBuf：
        /// 球体（中心原点；透视段相机消费）。
        #[pyfunction]
        fn gen_sphere(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let radius = args
                .args
                .first()
                .and_then(|o| ArgIntoFloat::try_from_object(vm, o.clone()).ok())
                .map(|f| f.into_float() as f32)
                .ok_or_else(|| vm.new_type_error("gen_sphere(radius, segments_u=16, segments_v=12)"))?;
            let su = kw_arg_int(&args, vm, "segments_u", 16)?;
            let sv = kw_arg_int(&args, vm, "segments_v", 12)?;
            let g = starfish_gfx::geometry::shape3d::sphere(
                glam::Vec3::ZERO,
                radius,
                su,
                sv,
                [1.0; 4],
            );
            Ok(geo_to_buf(g))
        }

        /// gen_plane(center, right, up, size) -> VertexBuf：任意朝向面片
        ///（center=[x,y,z]；right/up 朝向向量；size=[w,h]；透视段相机
        /// 消费）。
        #[pyfunction]
        fn gen_plane(center: Vec<f32>, right: Vec<f32>, up: Vec<f32>, size: Vec<f32>, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            if center.len() < 3 || right.len() < 3 || up.len() < 3 || size.len() < 2 {
                return Err(vm.new_type_error("gen_plane(center[3], right[3], up[3], size[2])"));
            }
            let g = starfish_gfx::geometry::shape3d::plane(
                glam::vec3(center[0], center[1], center[2]),
                glam::vec3(right[0], right[1], right[2]),
                glam::vec3(up[0], up[1], up[2]),
                glam::vec2(size[0], size[1]),
                [1.0; 4],
            );
            Ok(geo_to_buf(g))
        }

        /// gen_box(min, max) -> VertexBuf：轴对齐盒（min=[x,y,z] / max=
        /// [x,y,z] 对角点；透视段相机消费）。
        #[pyfunction]
        fn gen_box(min: Vec<f32>, max: Vec<f32>, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            if min.len() < 3 || max.len() < 3 {
                return Err(vm.new_type_error("gen_box(min[3], max[3])"));
            }
            let g = starfish_gfx::geometry::shape3d::cube(
                glam::vec3(min[0], min[1], min[2]),
                glam::vec3(max[0], max[1], max[2]),
                [1.0; 4],
            );
            Ok(geo_to_buf(g))
        }

        /// gen_polygon3d(points) -> VertexBuf：3D 多边形板（扁平
        /// [x,y,z × N]；Newell 法稳健基三角化；透视段相机消费）。
        #[pyfunction]
        fn gen_polygon3d(points: Vec<f32>, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            if points.len() % 3 != 0 || points.len() < 9 {
                return Err(vm.new_value_error(format!(
                    "gen_polygon3d 扁平长度须为 3 的倍数且至少 3 点，实际 {}",
                    points.len()
                )));
            }
            let pts: Vec<glam::Vec3> = points
                .chunks_exact(3)
                .map(|c| glam::vec3(c[0], c[1], c[2]))
                .collect();
            let g = starfish_gfx::geometry::shape3d::polygon_3d(&pts, [1.0; 4]);
            Ok(geo_to_buf(g))
        }

        /// gen_cylinder(radius, height, segments=24) -> VertexBuf：圆柱
        ///（中心原点 = 柱高中点；透视段相机消费）。
        #[pyfunction]
        fn gen_cylinder(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let p: Vec<f32> = args
                .args
                .iter()
                .map(|o| {
                    ArgIntoFloat::try_from_object(vm, o.clone()).map(|f| f.into_float() as f32)
                })
                .collect::<Result<Vec<f32>, _>>()
                .map_err(|_| vm.new_type_error("gen_cylinder(radius, height, segments=24)"))?;
            if p.len() < 2 {
                return Err(vm.new_type_error("gen_cylinder(radius, height, segments=24)"));
            }
            let seg = kw_arg_int(&args, vm, "segments", 24)?;
            let g = starfish_gfx::geometry::shape3d::cylinder(
                glam::vec3(0.0, -p[1] * 0.5, 0.0),
                p[0],
                p[1],
                seg,
                [1.0; 4],
            );
            Ok(geo_to_buf(g))
        }

        /// gen_cone(radius, height, segments=24) -> VertexBuf：圆锥（中心
        /// 原点 = 柱高中点；透视段相机消费）。
        #[pyfunction]
        fn gen_cone(args: FuncArgs, vm: &VirtualMachine) -> PyResult<PyVertexBuf> {
            use rustpython_vm::function::ArgIntoFloat;
            let p: Vec<f32> = args
                .args
                .iter()
                .map(|o| {
                    ArgIntoFloat::try_from_object(vm, o.clone()).map(|f| f.into_float() as f32)
                })
                .collect::<Result<Vec<f32>, _>>()
                .map_err(|_| vm.new_type_error("gen_cone(radius, height, segments=24)"))?;
            if p.len() < 2 {
                return Err(vm.new_type_error("gen_cone(radius, height, segments=24)"));
            }
            let seg = kw_arg_int(&args, vm, "segments", 24)?;
            let g = starfish_gfx::geometry::shape3d::cone(
                glam::vec3(0.0, -p[1] * 0.5, 0.0),
                p[0],
                p[1],
                seg,
                [1.0; 4],
            );
            Ok(geo_to_buf(g))
        }

        /// draw_surface(f, src, transform=None, src_rect=None, dst_rect=None,
        /// tint=None, blend=None, cull=None)：
        /// 画离屏面统一口（2026-11-27 接口破坏定约——uv/tf/centered 三口
        /// 合并）。约束：transform 与 dst_rect **互斥**（同给 = ValueError）。
        /// 路由真值表：
        /// - transform=Some → 全权矩阵（mat 变换中心锚 quad——不复合相机）
        /// - dst_rect=Some → SDL 式拉伸（[x,y,w,h] 目标逻辑像素）
        /// - src_rect=Some → UV 归一化 [u0,v0,u1,v1] 取块（可与其他组合）
        /// - 全 None → src 原始尺寸中心锚直绘
        /// tint：None = 不着色；Some = 与 src.blit_tint 复合（RGBA 0..255）
        /// blend/cull：本次调用的管线状态（None = alpha / 不剔除）
        #[derive(FromArgs)]
        struct DrawSurfaceArgs {
            f: PyObjectRef,
            src: PyRef<PySurface>,
            #[pyarg(any, optional)]
            transform: Option<PyRef<crate::rpy::pygame::math::math::PyMat4>>,
            /// UV 归一化 [u0,v0,u1,v1]
            #[pyarg(any, optional)]
            src_rect: Option<Vec<f64>>,
            /// 目标逻辑像素 [x,y,w,h]（与 transform 互斥）
            #[pyarg(any, optional)]
            dst_rect: Option<Vec<f64>>,
            #[pyarg(any, optional)]
            tint: Option<SfColor>,
            /// 混合档位（None = alpha）
            #[pyarg(any, optional)]
            blend: Option<String>,
            /// 剔除（None = none；none / back）
            #[pyarg(any, optional)]
            cull: Option<String>,
        }

        #[pyfunction]
        fn draw_surface(args: DrawSurfaceArgs, vm: &VirtualMachine) -> PyResult<()> {
            use starfish_pygame::render::DrawTarget;
            let state = DrawState::from_names(args.blend.as_deref(), args.cull.as_deref())
                .map_err(|e| vm.new_value_error(e))?;
            // 互斥校验（文档写死 + 代码报错）
            if args.transform.is_some() && args.dst_rect.is_some() {
                return Err(vm.new_value_error(
                    "draw_surface：transform 与 dst_rect 互斥（同给）",
                ));
            }
            let src = &args.src;
            src.surface.flush();
            let (sw, sh) = src.surface.size32();
            let (hw, hh) = (sw * 0.5, sh * 0.5);
            let tint = args.tint.map(|c| c.0).map(|px| {
                [
                    px[0] as f32 / 255.0,
                    px[1] as f32 / 255.0,
                    px[2] as f32 / 255.0,
                    px[3] as f32 / 255.0,
                ]
            });
            let mut color = src.surface.blit_tint();
            if let Some(t) = tint {
                color = [
                    color[0] * t[0],
                    color[1] * t[1],
                    color[2] * t[2],
                    color[3] * t[3],
                ];
            }
            // src_rect：UV 归一化 [u0,v0,u1,v1]（None = 整张）
            let uv = match &args.src_rect {
                Some(v) => {
                    if v.len() != 4 {
                        return Err(vm.new_value_error(format!(
                            "src_rect 长度须为 4（u0,v0,u1,v1），实际 {}",
                            v.len()
                        )));
                    }
                    [
                        v[0] as f32, v[1] as f32, v[2] as f32, v[3] as f32,
                    ]
                }
                None => [0.0, 0.0, 1.0, 1.0],
            };
            let uvs = [
                [uv[0], uv[1]],
                [uv[2], uv[1]],
                [uv[2], uv[3]],
                [uv[0], uv[3]],
            ];
            // corners：transform → mat 变换中心锚 quad；dst_rect → SDL
            // 式拉伸（[x,y,w,h] 目标逻辑像素左下角）；全 None → 中心锚直绘
            let corners: [[f32; 3]; 4] = match (&args.transform, &args.dst_rect) {
                (Some(m), _) => {
                    let m = m.0.get();
                    let c = |x: f32, y: f32| {
                        let p = m.transform_point3(glam::Vec3::new(x, y, 0.0));
                        [p.x, p.y, p.z]
                    };
                    [c(-hw, -hh), c(hw, -hh), c(hw, hh), c(-hw, hh)]
                }
                (_, Some(d)) => {
                    if d.len() != 4 {
                        return Err(vm.new_value_error(format!(
                            "dst_rect 长度须为 4（x,y,w,h），实际 {}",
                            d.len()
                        )));
                    }
                    let (x, y, w, h) =
                        (d[0] as f32, d[1] as f32, d[2] as f32, d[3] as f32);
                    [
                        [x, y, 0.0],
                        [x + w, y, 0.0],
                        [x + w, y + h, 0.0],
                        [x, y + h, 0.0],
                    ]
                }
                _ => [
                    [-hw, -hh, 0.0],
                    [hw, -hh, 0.0],
                    [hw, hh, 0.0],
                    [-hw, hh, 0.0],
                ],
            };
            // 三路由统一进 batch（同一 material 管线族——直绘 = 中心锚
            // quad 特例；mat/dst 已在 corners 生成端 CPU 烘）
            fn draw_quad<T: starfish_pygame::render::DrawTarget + ?Sized>(
                t: &T,
                corners: [[f32; 3]; 4],
                uvs: [[f32; 2]; 4],
                color: [f32; 4],
                tex: &starfish_pygame::render::Texture,
                state: DrawState,
            ) {
                t.with_batch(|b| {
                    b.push_quad_corners_3d_state(corners, uvs, color, Some(tex), state, false);
                });
            }
            let tex = src.surface.texture().clone();
            match route(&args.f, vm)? {
                Target::Surf(s) => draw_quad(&*s.surface, corners, uvs, color, &tex, state),
                Target::Active => bridge::with_session(vm, |t| {
                    draw_quad(t, corners, uvs, color, &tex, state)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    draw_quad(t, corners, uvs, color, &tex, state)
                })?,
            }
            Ok(())
        }

        // ── 3D 形状直调口已删（2026-11-27 用户定夺：只留一条通道）──
        // 3D 几何统一走 gen_*（生成 VertexBuf）+ draw_batch（渲染）；
        // 旧 cube / sphere / plane / cylinder / cone / polygon3d 全部移除。


    }
    // ── mask：位面（碰撞检测/像素级遮挡；pygame.mask 语义）────────
    // 构建 = from_surface（影子快路径三端同步零回读；GPU 出生面原生
    // 走一次性回读，Web 侧同步面对 GPU 出生面不提供——from_surface_async

pub(crate) use gfxdraw::module_def;
