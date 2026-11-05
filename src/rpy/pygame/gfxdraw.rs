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
        use crate::rpy::bridge;
        use starfish_pygame::render::DrawTarget;
        use rustpython_vm::{
            PyObjectRef, PyRef, PyResult, VirtualMachine,
            function::FuncArgs,
        };

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
