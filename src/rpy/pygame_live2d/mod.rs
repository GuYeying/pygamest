//! pygame_live2d —— Live2D/Cubism 绑定（阶段 4；独立顶层 Python 模块
//! ——pygame_spine 同范式：`import pygame_live2d` 直达）。
//!
//! 上游 = mocari v0.4.0（MIT，Eatgrapes/Mocari；上游仓已删——经 LunaMate
//! 内嵌副本溯源 vendored 进 pygame_live2d 兄弟 crate，wgpu 依赖恢复
//! upstream 原生形态）。
//!
//! 接口风格：上游词汇原生化（Live2D/Cubism 惯例 canvas / drawable /
//! parameter）。数据流：`load_model(path)` 静态数据面（meshes/textures/
//! canvas）→ `load_runtime(path, surface)` 参数驱动面 + **注入式渲染**
//! （用户创建 Surface 注入；`rt.draw(x, y, scale)` 全管线自理——蒙版
//! 纹理方案 GPU 落地在 crate render::pygst，本文件零渲染编排；产物 =
//! 全透明底人物图，gfxdraw.draw_surface 上屏/摆放）。
//!
//! 坐标系：Live2D 原生画布域（y 向下、原点画布左上）——与引擎中心系
//! 换算脚本侧自理（canvas_size + origin 可换算）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    types::Constructor, builtins::{PyStrRef, PyType},
};
use std::cell::RefCell;
use std::path::PathBuf;

#[pymodule(name = "pygame_live2d")]
pub mod pygame_live2d {
    use rustpython_derive::{pyclass, FromArgs};
    use rustpython_vm::{
        function::{ArgIntoFloat, FuncArgs},
        PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
        builtins::{PyStrRef, PyType},
    };
    use std::cell::RefCell;
    use std::path::PathBuf;

    // 内层模块与 crate 同名——`::` 全路径别名（pygame_spine 同款）
    use ::pygame_live2d as l2d;

    /// Live2DModel（静态数据面——canvas / meshes / textures）
    #[derive(PyPayload)]
    #[pyattr]
    #[pyclass(module = "pygame_live2d", name = "Live2DModel", unhashable = true)]
    pub(crate) struct PyLive2DModel {
        pub(crate) inner: l2d::assets::DefaultModel,
    }

    impl std::fmt::Debug for PyLive2DModel {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&format!(
                "Live2DModel(<{} meshes, {} textures>)",
                self.inner.meshes().len(),
                self.inner.textures().len()
            ))
        }
    }

    /// Live2DRuntime（参数驱动面——set_parameter / update_meshes / draw）
    #[derive(PyPayload)]
    #[pyattr]
    #[pyclass(module = "pygame_live2d", name = "Live2DRuntime", unhashable = true)]
    pub(crate) struct PyLive2DRuntime {
        pub(crate) inner: RefCell<l2d::assets::RuntimeModel>,
        /// 注入式渲染器（管线 crate 内自理——用户创建 Surface 注入，
        /// 每对象自持渲染目标 + Mg/C 蒙版工作面池 + 内容锚定）
        pub(crate) renderer: RefCell<l2d::render::pygst::Live2DRenderer>,
    }

    impl std::fmt::Debug for PyLive2DRuntime {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&format!(
                "Live2DRuntime(<{} params, {} drawables>)",
                self.inner.borrow().runtime().parameter_ids().len(),
                self.inner.borrow().runtime().drawable_ids().len()
            ))
        }
    }

    fn path_of(s: PyStrRef, vm: &VirtualMachine) -> PyResult<PathBuf> {
        let t = s
            .to_str()
            .ok_or_else(|| vm.new_value_error("路径含非 UTF-8 字符"))?;
        Ok(PathBuf::from(t))
    }

    /// load_model(path) -> Live2DModel：.model3.json 入口（moc3/纹理
    /// 相对路径按其目录解析）。
    #[pyfunction]
    fn load_model(path: PyStrRef, vm: &VirtualMachine) -> PyResult<PyRef<PyLive2DModel>> {
        let p = path_of(path, vm)?;
        match l2d::assets::load_model(&p) {
            Ok(m) => Ok(PyLive2DModel { inner: m }.into_ref(&vm.ctx)),
            Err(e) => Err(vm.new_value_error(format!("Live2D 模型加载失败: {e:?}"))),
        }
    }

    /// load_runtime(path, surface) -> Live2DRuntime：参数驱动形态 +
    /// 注入式渲染（surface = 用户创建的产物面——全透明底人物图，精灵
    /// 工作流：rt.draw() 画进 surface，gfxdraw.draw_surface 上屏）。
    #[pyfunction]
    fn load_runtime(
        path: PyStrRef,
        surface: PyRef<crate::rpy::pygame::pygame::PySurface>,
        vm: &VirtualMachine,
    ) -> PyResult<PyRef<PyLive2DRuntime>> {
        let p = path_of(path, vm)?;
        match l2d::assets::load_model_runtime(&p) {
            Ok(m) => {
                let renderer =
                    l2d::render::pygst::Live2DRenderer::new(surface.surface.clone(), &m);
                Ok(PyLive2DRuntime {
                    inner: RefCell::new(m),
                    renderer: RefCell::new(renderer),
                }
                .into_ref(&vm.ctx))
            }
            Err(e) => Err(vm.new_value_error(format!("Live2D 运行时加载失败: {e:?}"))),
        }
    }

    /// draw(x, y, scale) 收参（全可选——默认 = 居中 90% 画布利用率）
    #[derive(FromArgs)]
    struct Live2DDrawArgs {
        #[pyarg(any, optional)]
        x: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        y: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        scale: Option<ArgIntoFloat>,
    }

    /// mesh 元组组装（静态面/运行时面共用——positions/uvs 扁平 f32 +
    /// indices u16 + 纹理索引 + 不透明度）。
    fn mesh_tuple(
        vm: &VirtualMachine,
        m: &l2d::moc3::Moc3DrawableMesh,
    ) -> PyObjectRef {
        let pos: Vec<PyObjectRef> = m
            .vertices()
            .iter()
            .flat_map(|v| v.position())
            .map(|v| vm.ctx.new_float(v as f64).into())
            .collect();
        let uvs: Vec<PyObjectRef> = m
            .vertices()
            .iter()
            .flat_map(|v| v.uv())
            .map(|v| vm.ctx.new_float(v as f64).into())
            .collect();
        let idx: Vec<PyObjectRef> = m
            .indices()
            .iter()
            .map(|t| vm.ctx.new_int(*t as i64).into())
            .collect();
        vm.new_tuple((
            m.texture_index(),
            m.opacity() as f64,
            PyObjectRef::from(vm.ctx.new_list(pos)),
            PyObjectRef::from(vm.ctx.new_list(uvs)),
            PyObjectRef::from(vm.ctx.new_list(idx)),
        ))
        .into()
    }

    #[pyclass]
    impl PyLive2DModel {
        /// canvas_size() -> (w, h)：模型画布尺寸（Live2D 原生单位）。
        #[pymethod]
        fn canvas_size(&self) -> (f32, f32) {
            let c = self.inner.canvas();
            (c.width(), c.height())
        }

        /// mesh_count() -> int：drawable 网格数。
        #[pymethod]
        fn mesh_count(&self) -> usize {
            self.inner.meshes().len()
        }

        /// mesh(i) -> (texture_index, opacity, positions, uvs, indices)：
        /// 网格数据流（引擎带贴图 mesh draw 面的消费形态）。
        #[pymethod]
        fn mesh(&self, i: usize, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let m = self
                .inner
                .meshes()
                .get(i)
                .ok_or_else(|| vm.new_value_error(format!("网格索引越界: {i}")))?;
            Ok(mesh_tuple(vm, m))
        }

        /// texture_count() -> int：纹理数。
        #[pymethod]
        fn texture_count(&self) -> usize {
            self.inner.textures().len()
        }

        /// texture_info(i) -> (w, h)：纹理尺寸。
        #[pymethod]
        fn texture_info(&self, i: usize, vm: &VirtualMachine) -> PyResult<(u32, u32)> {
            let t = self
                .inner
                .textures()
                .get(i)
                .ok_or_else(|| vm.new_value_error(format!("纹理索引越界: {i}")))?;
            Ok((t.width(), t.height()))
        }

        /// texture_rgba(i) -> bytes：纹理原始 RGBA 字节（大对象——
        /// Surface 构建接取用；调试/上传用）。
        #[pymethod]
        fn texture_rgba(&self, i: usize, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let t = self
                .inner
                .textures()
                .get(i)
                .ok_or_else(|| vm.new_value_error(format!("纹理索引越界: {i}")))?;
            let bytes: Vec<PyObjectRef> = t
                .rgba()
                .iter()
                .map(|b| vm.ctx.new_int(*b as i64).into())
                .collect();
            Ok(PyObjectRef::from(vm.ctx.new_list(bytes)))
        }
    }

    #[pyclass]
    impl PyLive2DRuntime {
        /// canvas_size() -> (w, h)：模型画布尺寸。
        #[pymethod]
        fn canvas_size(&self) -> (f32, f32) {
            let c = self.inner.borrow().runtime().canvas();
            (c.width(), c.height())
        }

        /// parameter_ids() -> list[str]：可驱动参数 id 表。
        #[pymethod]
        fn parameter_ids(&self, vm: &VirtualMachine) -> PyObjectRef {
            let items: Vec<PyObjectRef> = self
                .inner
                .borrow()
                .runtime()
                .parameter_ids()
                .iter()
                .map(|s| vm.ctx.new_str(s.clone()).into())
                .collect();
            PyObjectRef::from(vm.ctx.new_list(items))
        }

        /// set_parameter_normalized(id, v) -> bool：归一参数驱动
        /// （0..1；id 不存在返回 False）。
        #[pymethod]
        fn set_parameter_normalized(
            &self,
            id: PyStrRef,
            v: ArgIntoFloat,
            vm: &VirtualMachine,
        ) -> PyResult<bool> {
            let t = id
                .to_str()
                .ok_or_else(|| vm.new_value_error("参数 id 含非 UTF-8 字符"))?;
            Ok(self
                .inner
                .borrow_mut()
                .runtime_mut()
                .set_parameter_normalized(t, v.into_float() as f32))
        }

        /// update_meshes()：参数 → 网格重建（每帧参数更新后调用；
        /// mesh(i) 读取重建产物）。
        #[pymethod]
        fn update_meshes(&self) {
            self.inner.borrow_mut().runtime_mut().update_meshes();
        }

        /// mesh_count() -> int / mesh(i)：数据面同 Live2DModel.mesh。
        #[pymethod]
        fn mesh_count(&self) -> usize {
            self.inner.borrow().runtime().meshes().len()
        }

        #[pymethod]
        fn mesh(&self, i: usize, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let sk = self.inner.borrow();
            let rt = sk.runtime();
            let m = rt
                .meshes()
                .get(i)
                .ok_or_else(|| vm.new_value_error(format!("网格索引越界: {i}")))?;
            Ok(mesh_tuple(vm, m))
        }

        /// draw(x=0.0, y=0.0, scale=0.9)：全管线自理——蒙版/masked/
        /// 整模型三段落注入面（产物=全透明底人物图）。坐标定约（与
        /// pygamest 中心系同构，全 0-1 相对注入面）：x/y=内容中心位置
        /// （0=面中心，±1=±半幅，y 向上）；scale=内容高/面高（>1 溢出
        /// 裁边=特写）。
        #[pymethod]
        fn draw(&self, args: Live2DDrawArgs, vm: &VirtualMachine) -> PyResult<()> {
            let model = self.inner.borrow();
            let renderer = self.renderer.borrow();
            renderer
                .draw(
                    &model,
                    args.x.map(|v| v.into_float() as f32).unwrap_or(0.0),
                    args.y.map(|v| v.into_float() as f32).unwrap_or(0.0),
                    args.scale.map(|v| v.into_float() as f32).unwrap_or(0.9),
                )
                .map_err(|e| vm.new_runtime_error(format!("live2d draw: {e}")))
        }

    }
        /// PyLive2DMotion（motion3 资产句柄）
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_live2d", name = "Live2DMotion", unhashable = true)]
        pub(crate) struct PyLive2DMotion {
            pub(crate) inner: std::sync::Arc<l2d::Motion3>,
        }

        impl std::fmt::Debug for PyLive2DMotion {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Live2DMotion(<motion3>)")
            }
        }

        #[pyclass]
        impl PyLive2DMotion {
            /// duration() -> float：motion 时长（秒）。
            #[pymethod]
            fn duration(&self) -> f32 {
                self.inner.meta().duration()
            }
        }

        /// PyLive2DMotionPlayer（motion 播放器——tick/apply 驱动参数）
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_live2d", name = "Live2DMotionPlayer", unhashable = true)]
        pub(crate) struct PyLive2DMotionPlayer {
            pub(crate) inner: RefCell<l2d::MotionPlayer>,
        }

        impl std::fmt::Debug for PyLive2DMotionPlayer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Live2DMotionPlayer(<motion>)")
            }
        }

        /// MotionPlayer 收参（looping 缺省 = 跟随 motion 自带 Loop 元数据）
        #[derive(FromArgs)]
        struct MotionPlayerArgs {
            motion: PyRef<PyLive2DMotion>,
            #[pyarg(any, optional)]
            looping: Option<bool>,
        }

        /// load_motion(path) -> Live2DMotion：.motion3.json 加载。
        #[pyfunction]
        fn load_motion(path: PyStrRef, vm: &VirtualMachine) -> PyResult<PyRef<PyLive2DMotion>> {
            let p = path_of(path, vm)?;
            match l2d::motion::load_motion(&p) {
                Ok(m) => Ok(PyLive2DMotion {
                    inner: std::sync::Arc::new(m),
                }
                .into_ref(&vm.ctx)),
                Err(e) => Err(vm.new_value_error(format!("motion 加载失败: {e:?}"))),
            }
        }

        /// Live2DMotionPlayer(motion, looping=None)：播放器构造。
        #[pyfunction]
        fn motion_player(
            args: MotionPlayerArgs,
            vm: &VirtualMachine,
        ) -> PyResult<PyRef<PyLive2DMotionPlayer>> {
            let player = match args.looping {
                Some(l) => l2d::MotionPlayer::with_looping(args.motion.inner.clone(), l),
                None => l2d::MotionPlayer::new(args.motion.inner.clone()),
            };
            Ok(PyLive2DMotionPlayer {
                inner: RefCell::new(player),
            }
            .into_ref(&vm.ctx))
        }

        #[pyclass]
        impl PyLive2DMotionPlayer {
            /// tick(dt)：推进时间轴（秒）。
            #[pymethod]
            fn tick(&self, dt: ArgIntoFloat) {
                self.inner.borrow_mut().tick(dt.into_float() as f32);
            }

            /// apply(model)：motion 参数写进 runtime（写后 update_meshes）。
            #[pymethod]
            fn apply(&self, model: PyRef<PyLive2DRuntime>) {
                let mut m = model.inner.borrow_mut();
                self.inner.borrow().apply(m.runtime_mut());
            }

            /// is_finished() -> bool：非循环 motion 是否播完。
            #[pymethod]
            fn is_finished(&self) -> bool {
                self.inner.borrow().is_finished()
            }

            /// restart()：回零。
            #[pymethod]
            fn restart(&self) {
                self.inner.borrow_mut().restart();
            }

            /// time() -> float：当前时间轴。
            #[pymethod]
            fn time(&self) -> f32 {
                self.inner.borrow().time()
            }

            /// set_weight(w)：混合权重（0..1）。
            #[pymethod]
            fn set_weight(&self, w: ArgIntoFloat) {
                self.inner.borrow_mut().set_weight(w.into_float() as f32);
            }
        }
}

pub(crate) use self::pygame_live2d::{module_def, PyLive2DMotion, PyLive2DMotionPlayer, PyLive2DModel, PyLive2DRuntime};
