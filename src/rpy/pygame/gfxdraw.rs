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
        use rustpython_derive::FromArgs;
        use crate::rpy::bridge;
        use starfish_pygame::render::DrawTarget;
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
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        use starfish_pygame::render::DrawTarget;
        let sk = skeleton.inner.borrow();
        let cmds = pygame_spine::render::render(&sk);
        fn draw_cmds<T: DrawTarget + ?Sized>(
            t: &T,
            cmds: &[pygame_spine::RenderCommand],
            pages: &[std::sync::Arc<starfish_pygame::render::Surface>],
        ) {
            for c in cmds {
                let Some(page) = pages.get(c.page) else {
                    continue; // 图集页越界：跳过（数据面校验在 Python 侧）
                };
                let positions: Vec<f32> =
                    c.positions.iter().flat_map(|p| [p.x, p.y]).collect();
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
                draw_cmds(&*s.surface, &cmds, &page_surfaces);
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                draw_cmds(t, &cmds, &page_surfaces);
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                draw_cmds(t, &cmds, &page_surfaces);
                Ok(())
            })?,
        }
    }

    /// draw_live2d(f, model)：Live2D 模型渲染——RuntimeModel 网格重建
    /// 产物 → mesh 上屏（纹理惰性 from_rgba8 缓存于 model；uv 翻 v +
    /// y 翻转：Live2D 画布域 y 向下 → 引擎中心系 y 向上；opacity 乘入
    /// 顶点色）。
    #[cfg(feature = "live2d")]
    #[derive(FromArgs)]
    struct Live2DDrawArgs {
        model: PyRef<crate::rpy::pygame_live2d::pygame_live2d::PyLive2DRuntime>,
        #[pyarg(any, optional)]
        scale: Option<ArgIntoFloat>,
    }

    #[cfg(feature = "live2d")]
    #[pyfunction]
    fn draw_live2d(f: PyObjectRef, args: Live2DDrawArgs, vm: &VirtualMachine) -> PyResult<()> {
        let model = args.model;
        let scale = args.scale.map(|v| v.into_float() as f32).unwrap_or(1.0);
        use ::pygame_live2d as l2d_crate;
        use starfish_pygame::render::{DrawTarget, Surface as SfSurface};
        // 纹理惰性构建（一次——2048{b2} RGBA 每帧重建不可接受）
        {
            let mut tc = model.textures.borrow_mut();
            if tc.is_none() {
                let rt = model.inner.borrow();
                let surfaces: Vec<std::sync::Arc<SfSurface>> = rt
                    .textures()
                    .iter()
                    .map(|t| {
                        std::sync::Arc::new(SfSurface::from_rgba8(
                            (t.width(), t.height()),
                            t.rgba(),
                        ))
                    })
                    .collect();
                *tc = Some(surfaces);
            }
        }
        let texs = model.textures.borrow();
        let tex_list = texs.as_ref().expect("纹理缓存刚确保");
        let rt = model.inner.borrow();
        // 按 Cubism RenderOrder 排序（drawable 索引序 ≠ 绘制序——运行时
        // 顺序动态变化；不排序 = 手臂/头发层级错乱、该在背后的画到前面）
        let mut ordered: Vec<_> = rt.runtime().meshes().iter().enumerate().collect();
        ordered.sort_by_key(|(_, m)| m.render_order());
        fn draw_meshes<T: DrawTarget + ?Sized>(
            t: &T,
            meshes: &[(usize, &l2d_crate::moc3::Moc3DrawableMesh)],
            tex_list: &[std::sync::Arc<SfSurface>],
            scale: f32,
        ) {
            if std::env::var("L2D_DEBUG").is_ok() {
                let xs: Vec<f32> = meshes.iter().flat_map(|(_, m)| m.vertices().iter().map(|v| v.position()[0])).collect();
                let ys: Vec<f32> = meshes.iter().flat_map(|(_, m)| m.vertices().iter().map(|v| v.position()[1] * scale)).collect();
                eprintln!("[dbg] live2d 顶点 x {:?}..{:?} y {:?}..{:?} scale={}", xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max), ys.iter().cloned().fold(f32::MAX, f32::min), ys.iter().cloned().fold(f32::MIN, f32::max), scale);
            }
            for (_, m) in meshes {
                // masked drawable（眉眼口等蒙版裁剪部位）——stencil 蒙版面
                // 未落地前跳过（画错位比缺部位更诡异）
                if !m.masks().is_empty() {
                    continue;
                }
                let tex_i = m.texture_index();
                let Some(tex) = tex_list.get(tex_i as usize) else {
                    continue;
                };
                // mocari 顶点已是归一化 y 向上域——直传零翻转
                let positions: Vec<f32> = m
                    .vertices()
                    .iter()
                    .flat_map(|v| [v.position()[0] * scale, v.position()[1] * scale])
                    .collect();
                // mocari uv = 纹理原生域直传（零翻转）
                let uvs: Vec<f32> = m
                    .vertices()
                    .iter()
                    .flat_map(|v| [v.uv()[0], v.uv()[1]])
                    .collect();
                let opacity = m.opacity();
                let indices = m.indices().clone();
                let rgba = [1.0, 1.0, 1.0, opacity.clamp(0.0, 1.0)];
                let tex = tex_list.get(tex_i as usize);
                if let Some(tex) = tex {
                    t.draw_mesh(tex, &positions, &uvs, &indices, rgba);
                }
            }
        }
        match route(&f, vm)? {
            Target::Surf(s) => {
                draw_meshes(&*s.surface, &ordered, tex_list, scale);
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                draw_meshes(t, &ordered, tex_list, scale);
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                draw_meshes(t, &ordered, tex_list, scale);
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
                t.push_points(
                    BatchTopology::Lines,
                    None,
                    &[(px(jx) as f32, py(jy) as f32), (px(ppos.0) as f32, py(ppos.1) as f32)],
                    [1.0, 0.9, 0.3, 1.0],
                );
            }
        }
        match route(&f, vm)? {
            Target::Surf(s2) => {
                draw_bones(&*s2.surface, &pos, &parents, s, ox, oy);
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                draw_bones(t, &pos, &parents, s, ox, oy);
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                draw_bones(t, &pos, &parents, s, ox, oy);
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
        /// 仅本会话生效）
        #[pyfunction]
        fn set_camera(
            f: PyObjectRef,
            camera: PyRef<crate::rpy::pygame::render::render::PyCamera>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.with_batch(|b| b.set_mvp(Some(camera.0.mvp()))),
                Target::Active => bridge::with_session(vm, |t| t.set_camera(&camera.0))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.set_camera(&camera.0))?,
            }
            Ok(())
        }

        /// reset_camera(f)：回 Auto 兜底相机（按目标尺寸现算）
        #[pyfunction]
        fn reset_camera(f: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.with_batch(|b| b.reset_mvp()),
                Target::Active => bridge::with_session(vm, |t| t.reset_camera())?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.reset_camera())?,
            }
            Ok(())
        }

        /// set_transform(f, model)：会话变换（后续绘制顶点按此矩阵 CPU 烘）
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

        /// set_color(f)：one-shot 绘制色（下一次 gfxdraw 图元用此色；
        /// draw 后自动重置回默认紫色）
        #[pyfunction]
        fn set_color(
            f: PyObjectRef,
            color: SfColor,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            let c = [
                color.0[0] as f32 / 255.0,
                color.0[1] as f32 / 255.0,
                color.0[2] as f32 / 255.0,
                color.0[3] as f32 / 255.0,
            ];
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.set_draw_color(c),
                Target::Active => bridge::with_session(vm, |t| t.set_draw_color(c))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.set_draw_color(c))?,
            }
            Ok(())
        }

        /// set_wireframe(f, enabled=True)：one-shot 线框模式（下一次图元
        /// 渲染为三角网格边线——呈现原始三角化的边）
        #[pyfunction]
        fn set_wireframe(
            f: PyObjectRef,
            args: FuncArgs,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            use rustpython_vm::convert::TryFromObject;
            let enabled = args.args.first()
                .and_then(|o| bool::try_from_object(vm, o.clone()).ok())
                .unwrap_or(true);
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.set_draw_wireframe(enabled),
                Target::Active => bridge::with_session(vm, |t| t.set_draw_wireframe(enabled))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.set_draw_wireframe(enabled))?,
            }
            Ok(())
        }

        /// draw_surface(f, src)：画离屏面（位置/变换由 set_transform 承载）
        #[pyfunction]
        fn draw_surface(f: PyObjectRef, src: PyRef<PySurface>, vm: &VirtualMachine) -> PyResult<()> {
            use starfish_pygame::render::DrawTarget;
            match route(&f, vm)? {
                Target::Surf(s) => DrawTarget::draw_surface(&*s.surface, &src.surface),
                Target::Active => bridge::with_session(vm, |t| DrawTarget::draw_surface(t, &src.surface))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| DrawTarget::draw_surface(t, &src.surface))?,
            }
            Ok(())
        }

        /// draw_surface_centered(f, src, center)：中心锚画面
        #[pyfunction]
        fn draw_surface_centered(
            f: PyObjectRef,
            src: PyRef<PySurface>,
            center: crate::rpy::args::SfPoint,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            use starfish_pygame::render::DrawTarget;
            match route(&f, vm)? {
                Target::Surf(s) => DrawTarget::draw_surface_centered(
                    &*s.surface, &src.surface, (center.0 as f32, center.1 as f32)),
                Target::Active => bridge::with_session(vm, |t| DrawTarget::draw_surface_centered(
                    t, &src.surface, (center.0 as f32, center.1 as f32)))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| DrawTarget::draw_surface_centered(
                    t, &src.surface, (center.0 as f32, center.1 as f32)))?,
            }
            Ok(())
        }

        /// draw_surface_uv(f, src, uv, tint=None)：子区域画面（area 语义继任）
        #[pyfunction]
        fn draw_surface_uv(
            f: PyObjectRef,
            src: PyRef<PySurface>,
            uv: Vec<f32>,
            tint: Option<SfColor>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            use starfish_pygame::render::DrawTarget;
            match route(&f, vm)? {
                Target::Surf(s) => DrawTarget::draw_surface_uv(
                    &*s.surface, &src.surface,
                    [uv[0], uv[1], uv[2], uv[3]],
                    tint.map(|c| c.0).unwrap_or([255, 255, 255, 255])),
                Target::Active => bridge::with_session(vm, |t| DrawTarget::draw_surface_uv(
                    t, &src.surface,
                    [uv[0], uv[1], uv[2], uv[3]],
                    tint.map(|c| c.0).unwrap_or([255, 255, 255, 255])))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| DrawTarget::draw_surface_uv(
                    t, &src.surface,
                    [uv[0], uv[1], uv[2], uv[3]],
                    tint.map(|c| c.0).unwrap_or([255, 255, 255, 255])))?,
            }
            Ok(())
        }

        /// draw_surface_tf(f, src, mat, tint=None)：显式矩阵画面（mat 全权，
        /// 不复合会话 transform）
        #[pyfunction]
        fn draw_surface_tf(
            f: PyObjectRef,
            src: PyRef<PySurface>,
            mat: PyRef<crate::rpy::pygame::math::math::PyMat4>,
            tint: Option<SfColor>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            use starfish_pygame::render::DrawTarget;
            let tint = tint.map(|c| c.0).map(|px| [px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0, px[3] as f32 / 255.0]);
            match route(&f, vm)? {
                Target::Surf(s) => DrawTarget::draw_surface_tf(
                    &*s.surface, &src.surface, mat.0.get(), tint),
                Target::Active => bridge::with_session(vm, |t| DrawTarget::draw_surface_tf(
                    t, &src.surface, mat.0.get(), tint))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| DrawTarget::draw_surface_tf(
                    t, &src.surface, mat.0.get(), tint))?,
            }
            Ok(())
        }

        /// pixel(f, x, y)
        #[pyfunction]
        fn pixel(f: PyObjectRef, x: i32, y: i32, vm: &VirtualMachine) -> PyResult<()> {
                match route(&f, vm)? {
                Target::Surf(s) => {
                    starfish_pygame::draw::rect(&*s.surface,
                        [x, y, 1, 1], 0)
                }
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::rect(t,
                        [x, y, 1, 1], 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::rect(t,
                        [x, y, 1, 1], 0)
                })?,
            };
            Ok(())
        }

        /// hline(f, x1, x2, y)
        #[pyfunction]
        fn hline(
            f: PyObjectRef,
            x1: i32,
            x2: i32,
            y: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            line(f, x1, y, x2, y, vm)
        }

        /// vline(f, x, y1, y2)
        #[pyfunction]
        fn vline(
            f: PyObjectRef,
            x: i32,
            y1: i32,
            y2: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            line(f, x, y1, x, y2, vm)
        }

        /// line(f, x1, y1, x2, y2)
        #[pyfunction]
        fn line(
            f: PyObjectRef,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::line(&*s.surface, (x1, y1), (x2, y2), 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::line(t, (x1, y1), (x2, y2), 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::line(t, (x1, y1), (x2, y2), 1)
                })?,
            };
            Ok(())
        }

        /// aaline(f, x1, y1, x2, y2)：抗锯齿线 = 兼容别名
        ///（AA 由 wgpu 线性采样承载，同实现）。
        #[pyfunction]
        fn aaline(
            f: PyObjectRef,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            line(f, x1, y1, x2, y2, vm)
        }

        /// circle(f, x, y, radius)：1px 描边圆。
        #[pyfunction]
        fn circle(
            f: PyObjectRef,
            x: i32,
            y: i32,
            radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let r = radius.max(0) as u32;
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::circle(&*s.surface, (x, y), r, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::circle(t, (x, y), r, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::circle(t, (x, y), r, 1)
                })?,
            };
            Ok(())
        }

        /// aacircle(f, x, y, radius)：抗锯齿圆 = 兼容别名。
        #[pyfunction]
        fn aacircle(
            f: PyObjectRef,
            x: i32,
            y: i32,
            radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            circle(f, x, y, radius, vm)
        }

        /// filled_circle(f, x, y, radius)：填充圆。
        #[pyfunction]
        fn filled_circle(
            f: PyObjectRef,
            x: i32,
            y: i32,
            radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let r = radius.max(0) as u32;
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::circle(&*s.surface, (x, y), r, 0),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::circle(t, (x, y), r, 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::circle(t, (x, y), r, 0)
                })?,
            };
            Ok(())
        }

        /// ellipse(f, x, y, rx, ry)：1px 描边椭圆（中心 + 半径轴）。
        #[pyfunction]
        fn ellipse(
            f: PyObjectRef,
            x: i32,
            y: i32,
            rx: i32,
            ry: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let rect = [x - rx, y - ry, rx.max(0) * 2, ry.max(0) * 2];
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::ellipse(&*s.surface, rect, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::ellipse(t, rect, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::ellipse(t, rect, 1)
                })?,
            };
            Ok(())
        }

        /// aaellipse(f, x, y, rx, ry)：抗锯齿椭圆 = 兼容别名。
        #[pyfunction]
        fn aaellipse(
            f: PyObjectRef,
            x: i32,
            y: i32,
            rx: i32,
            ry: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            ellipse(f, x, y, rx, ry, vm)
        }

        /// filled_ellipse(f, x, y, rx, ry)：填充椭圆。
        #[pyfunction]
        fn filled_ellipse(
            f: PyObjectRef,
            x: i32,
            y: i32,
            rx: i32,
            ry: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let rect = [x - rx, y - ry, rx.max(0) * 2, ry.max(0) * 2];
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::ellipse(&*s.surface, rect, 0),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::ellipse(t, rect, 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::ellipse(t, rect, 0)
                })?,
            };
            Ok(())
        }

        /// arc(f, x, y, radius, start_angle, stop_angle)：椭圆弧
        ///（**度数制**——SDL_gfx 惯例；1px 描边，v1 简化）。
        #[pyfunction]
        fn arc(
            f: PyObjectRef,
            x: i32,
            y: i32,
            radius: i32,
            start_angle: i32,
            stop_angle: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let rect = [x - radius, y - radius, radius.max(0) * 2, radius.max(0) * 2];
            let (s, e) = (
                (start_angle as f32).to_radians(),
                (stop_angle as f32).to_radians(),
            );
            match route(&f, vm)? {
                Target::Surf(sfc) => starfish_pygame::draw::arc(&*sfc.surface, rect, s, e, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::arc(t, rect, s, e, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::arc(t, rect, s, e, 1)
                })?,
            };
            Ok(())
        }

        /// trigon 坐标组（9 参超 RP 原生函数实现上限——FromArgs 结构体
        /// 按声明序扁平展开，Python 签名不变）
        #[derive(rustpython_derive::FromArgs)]
        pub(crate) struct TrigonArgs {
            #[pyarg(any)]
            pub(crate) x1: i32,
            #[pyarg(any)]
            pub(crate) y1: i32,
            #[pyarg(any)]
            pub(crate) x2: i32,
            #[pyarg(any)]
            pub(crate) y2: i32,
            #[pyarg(any)]
            pub(crate) x3: i32,
            #[pyarg(any)]
            pub(crate) y3: i32,
        }

        /// trigon(f, x1, y1, x2, y2, x3, y3)：1px 描边三角。
        #[pyfunction]
        fn trigon(f: PyObjectRef, args: TrigonArgs, vm: &VirtualMachine) -> PyResult<()> {
            filled_polygon3(&f, args.x1, args.y1, args.x2, args.y2, args.x3, args.y3, vm)?;
            // 描边三角 = 填充 + 1px 轮廓（gfxdraw trigon 语义）
            polygon3(&f, args.x1, args.y1, args.x2, args.y2, args.x3, args.y3, vm)
        }

        /// aatrigon(...)：抗锯齿三角 = 兼容别名（同 trigon 组合）。
        #[pyfunction]
        fn aatrigon(f: PyObjectRef, args: TrigonArgs, vm: &VirtualMachine) -> PyResult<()> {
            trigon(f, args, vm)
        }

        /// filled_trigon(f, x1, y1, x2, y2, x3, y3)：填充三角。
        #[pyfunction]
        fn filled_trigon(f: PyObjectRef, args: TrigonArgs, vm: &VirtualMachine) -> PyResult<()> {
            filled_polygon3(&f, args.x1, args.y1, args.x2, args.y2, args.x3, args.y3, vm)
        }

        #[allow(clippy::too_many_arguments)]
        fn filled_polygon3(
            f: &PyObjectRef,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            x3: i32,
            y3: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let pts = SfPoints(vec![(x1, y1), (x2, y2), (x3, y3)]);
            match route(f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::polygon(&*s.surface, &pts.0, 0),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &pts.0, 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &pts.0, 0)
                })?,
            };
            Ok(())
        }

        #[allow(clippy::too_many_arguments)]
        fn polygon3(
            f: &PyObjectRef,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            x3: i32,
            y3: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let pts = SfPoints(vec![(x1, y1), (x2, y2), (x3, y3)]);
            match route(f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::polygon(&*s.surface, &pts.0, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &pts.0, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &pts.0, 1)
                })?,
            };
            Ok(())
        }

        /// polygon(f, points)：1px 描边多边形（points = [(x, y), ...]）。
        #[pyfunction]
        fn polygon(f: PyObjectRef, points: SfPoints, vm: &VirtualMachine) -> PyResult<()> {
                match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::polygon(&*s.surface, &points.0, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &points.0, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &points.0, 1)
                })?,
            };
            Ok(())
        }

        /// aapolygon(f, points)：抗锯齿多边形 = 兼容别名。
        #[pyfunction]
        fn aapolygon(f: PyObjectRef, points: SfPoints, vm: &VirtualMachine) -> PyResult<()> {
            polygon(f, points, vm)
        }

        /// filled_polygon(f, points)：填充多边形。
        #[pyfunction]
        fn filled_polygon(
            f: PyObjectRef,
            points: SfPoints,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::polygon(&*s.surface, &points.0, 0),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &points.0, 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &points.0, 0)
                })?,
            };
            Ok(())
        }

        /// rectangle(f, rect)：1px 描边矩形（rect = (left, top, w, h)）。
        #[pyfunction]
        fn rectangle(f: PyObjectRef, rect: SfRect, vm: &VirtualMachine) -> PyResult<()> {
                let r = [rect.x, rect.y, rect.w, rect.h];
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::rect(&*s.surface, r, 1),
                Target::Active => bridge::with_session(vm, |t| starfish_pygame::draw::rect(t, r, 1))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| starfish_pygame::draw::rect(t, r, 1))?,
            };
            Ok(())
        }

        /// box(f, rect)：填充矩形。
        #[pyfunction(name = "box")]
        fn box_(f: PyObjectRef, rect: SfRect, vm: &VirtualMachine) -> PyResult<()> {
                let r = [rect.x, rect.y, rect.w, rect.h];
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::rect(&*s.surface, r, 0),
                Target::Active => bridge::with_session(vm, |t| starfish_pygame::draw::rect(t, r, 0))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| starfish_pygame::draw::rect(t, r, 0))?,
            };
            Ok(())
        }

        /// rounded_rectangle(f, rect, radius)：垫底——圆角语义
        /// v1 未建（glam 无 SDF 圆角管线），降级为直角描边矩形。
        #[pyfunction]
        fn rounded_rectangle(
            f: PyObjectRef,
            rect: SfRect,
            _radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            rectangle(f, rect, vm)
        }

        /// rounded_box(f, rect, radius)：垫底——降级为填充矩形。
        #[pyfunction]
        fn rounded_box(
            f: PyObjectRef,
            rect: SfRect,
            _radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            box_(f, rect, vm)
        }
    }

    // ── mask：位面（碰撞检测/像素级遮挡；pygame.mask 语义）────────
    // 构建 = from_surface（影子快路径三端同步零回读；GPU 出生面原生
    // 走一次性回读，Web 侧同步面对 GPU 出生面不提供——from_surface_async

pub(crate) use gfxdraw::module_def;
