//! pygame_spine —— Spine 2D 骨骼动画绑定（阶段 4；独立顶层 Python 模块
//! ——pygame_gui / pygame_physics 同范式：`import pygame_spine` 直达）。
//!
//! 接口风格定约：**上游词汇原生化**（Spine 惯例 SkeletonData /
//! Skeleton / AnimationState 三件套；不 pygame 化）。数据流：
//! `load_json / load_binary / demo_skeleton` → `Skeleton(data)` →
//! `AnimationState`（set_animation / update / apply）→ `render_commands`
//! （RenderCommand 流——positions/uvs/triangles/page 透传，引擎侧带贴图
//! mesh draw 面落地后接 GPU；v1 为数据面出口）。
//!
//! 坐标系：Spine 原生 y 向上（与引擎中心系同构——零换算直出）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    types::{Constructor, Representable},
    builtins::{PyModule, PyStrRef, PyType},
};
use std::cell::RefCell;
use std::sync::Arc;

#[pymodule(name = "pygame_spine")]
    pub mod pygame_spine {
        use rustpython_derive::pyclass;
        use rustpython_vm::{
            function::ArgIntoFloat, PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
            types::Constructor, builtins::{PyStrRef, PyType},
        };
        use std::cell::RefCell;
        use std::sync::Arc;
        // 内层模块与上游 crate 同名（python 模块名 = crate 名定约）——
        // `::` 全路径别名破名空间冲突（模块内裸 `pygame_spine::` 会解析
        // 到自身）
        use ::pygame_spine as sp;

        // ── 类型声明（RP 宏要求 struct 与方法收集块同宿）──────────────

        /// SkeletonData（资产共享面——Arc 底；find_animation 检索动画）
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_spine", name = "SkeletonData", unhashable = true)]
        pub(crate) struct PySpineSkeletonData {
            /// Arc 可内可变（bind_atlas 期 Arc::get_mut 独占改写）
            pub(crate) data: RefCell<Arc<sp::SkeletonData>>,
        }

        impl std::fmt::Debug for PySpineSkeletonData {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&format!(
                    "SkeletonData(<{} bones, {} animations>)",
                    self.data.borrow().bones.len(),
                    self.data.borrow().animations.len()
                ))
            }
        }

        impl Constructor for PySpineSkeletonData {
            type Args = ();
            fn py_new(
                _cls: &Py<PyType>,
                _args: (),
                vm: &VirtualMachine,
            ) -> PyResult<Self> {
                Err(vm.new_type_error("SkeletonData 不可直呼构造——经 load_json / load_binary / demo_skeleton"))
            }
        }

        /// Skeleton（实例姿态面——骨骼世界变换查询 / 皮肤 / 平移缩放）
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_spine", name = "Skeleton", unhashable = true)]
        pub(crate) struct PySpineSkeleton {
            pub(crate) inner: RefCell<sp::Skeleton>,
            /// 渲染 scratch 复用（chine render_with——每帧免重分配命令缓冲）
            pub(crate) scratch: RefCell<sp::clip::RenderScratch>,
        }

        impl std::fmt::Debug for PySpineSkeleton {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Skeleton(<posed rig>)")
            }
        }

        impl Constructor for PySpineSkeleton {
            type Args = PyRef<PySpineSkeletonData>;
            fn py_new(
                _cls: &Py<PyType>,
                data: PyRef<PySpineSkeletonData>,
                _vm: &VirtualMachine,
            ) -> PyResult<Self> {
                Ok(Self {
                    inner: RefCell::new(sp::Skeleton::new(data.data.borrow().clone())),
                scratch: RefCell::new(sp::clip::RenderScratch::new()),
                })
            }
        }

        /// AnimationState（播放状态机——set_animation / update / apply）
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_spine", name = "AnimationState", unhashable = true)]
        pub(crate) struct PySpineState {
            pub(crate) inner: RefCell<sp::AnimationState>,
        }

        impl std::fmt::Debug for PySpineState {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("AnimationState(<tracks>)")
            }
        }

        impl Constructor for PySpineState {
            type Args = ();
            fn py_new(
                _cls: &Py<PyType>,
                _args: (),
                _vm: &VirtualMachine,
            ) -> PyResult<Self> {
                Ok(Self {
                    inner: RefCell::new(sp::AnimationState::new()),
                })
            }
        }

        // ── 模块函数 ─────────────────────────────────────────────

        /// load_json(text) -> SkeletonData：Spine JSON 导入（导出需含骨骼
        /// 数据；动画/atlas 分离文件各自再进）。
        #[pyfunction]
        fn load_json(text: PyStrRef, vm: &VirtualMachine) -> PyResult<PyRef<PySpineSkeletonData>> {
            let t = text
                .to_str()
                .ok_or_else(|| vm.new_value_error("文本含非 UTF-8 字符"))?;
            match sp::load::from_json(t) {
                Ok(data) => Ok(PySpineSkeletonData { data: RefCell::new(Arc::new(data)) }.into_ref(&vm.ctx)),
                Err(e) => Err(vm.new_value_error(format!("Spine JSON 解析失败: {e:?}"))),
            }
        }

        /// load_binary(bytes) -> SkeletonData：Spine 二进制导入（.skel）。
        #[pyfunction]
        fn load_binary(bytes: Vec<u8>, vm: &VirtualMachine) -> PyResult<PyRef<PySpineSkeletonData>> {
            match sp::binary::from_binary(&bytes) {
                Ok(data) => Ok(PySpineSkeletonData { data: RefCell::new(Arc::new(data)) }.into_ref(&vm.ctx)),
                Err(e) => Err(vm.new_value_error(format!("Spine 二进制解析失败: {e:?}"))),
            }
        }

        /// demo_skeleton() -> SkeletonData：程序化三骨链（root→arm→hand，
        /// 偏移 50/40——零资产测试链）。
        #[pyfunction]
        fn demo_skeleton(vm: &VirtualMachine) -> PyRef<PySpineSkeletonData> {
            PySpineSkeletonData {
                data: RefCell::new(sp::demo_skeleton()),
            }
            .into_ref(&vm.ctx)
        }

        #[pyclass(with(Constructor))]
        impl PySpineSkeletonData {
            /// find_animation(name) -> Animation | None（经
            /// AnimationState.set_animation 播放）。
            #[pymethod]
            fn find_animation(
                zelf: PyRef<Self>,
                name: PyStrRef,
                vm: &VirtualMachine,
            ) -> PyResult<Option<PyObjectRef>> {
                let t = name
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("文本含非 UTF-8 字符"))?;
                Ok(zelf
                    .data
                    .borrow()
                    .animations
                    .iter()
                    .find(|a| a.name() == t)
                    .cloned()
                    .map(|a| PySpineAnimation(a).into_ref(&vm.ctx).into()))
            }

            /// bone_count() -> int：骨骼数。
            #[pymethod]
            fn bone_count(&self) -> usize {
                self.data.borrow().bones.len()
            }

            /// bone_name(i) -> str：骨骼名。
            #[pymethod]
            fn bone_name(&self, i: usize, vm: &VirtualMachine) -> PyResult<String> {
                self.data.borrow().bones.get(i).map(|b| b.name.clone()).ok_or_else(|| {
                    vm.new_value_error(format!("骨骼索引越界: {i}"))
                })
            }
        }

        /// Animation（find_animation 产物——set_animation 直入状态机）。
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame_spine", name = "Animation", unhashable = true)]
        pub(crate) struct PySpineAnimation(pub(crate) Arc<sp::Animation>);

        impl std::fmt::Debug for PySpineAnimation {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Animation(<timelines>)")
            }
        }

        #[pyclass]
        impl PySpineAnimation {
            /// duration() -> float：动画时长（秒）。
            #[pymethod]
            fn duration(&self) -> f32 {
                self.0.duration()
            }

            /// name() -> str：动画名。
            #[pymethod]
            fn name(&self, vm: &VirtualMachine) -> PyResult<String> {
                Ok(self.0.name().to_owned())
            }
        }

        #[pyclass(with(Constructor))]
        impl PySpineSkeleton {
            /// set_skin(name)：激活皮肤（未知名 ValueError）。
            #[pymethod]
            fn set_skin(&self, name: PyStrRef, vm: &VirtualMachine) -> PyResult<()> {
                let t = name
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("文本含非 UTF-8 字符"))?;
                // 非法名校验（定约：未知名 ValueError——静默 no-op 会掩盖拼写错误）
                if self.inner.borrow().data().find_skin(t).is_none() {
                    return Err(vm.new_value_error(format!("皮肤不存在: {t}")));
                }
                self.inner.borrow_mut().set_skin(t);
                Ok(())
            }

            /// update_world_transform()：世界变换解算（pose 变更后、查询
            /// 前调用）。
            #[pymethod]
            fn update_world_transform(&self) {
                self.inner.borrow_mut().update_world_transform();
            }

            /// bone_world(i) -> (x, y, a, b, c, d)：骨骼世界变换（2×3 仿射
            /// 矩阵 + 世界原点——y 向上，与引擎中心系同构零换算）。
            #[pymethod]
            fn bone_world(&self, i: usize, vm: &VirtualMachine) -> PyResult<(f32, f32, f32, f32, f32, f32)> {
                let sk = self.inner.borrow();
                let b = sk
                    .bone(i)
                    .ok_or_else(|| vm.new_value_error(format!("骨骼索引越界: {i}")))?;
                Ok((b.world_x(), b.world_y(), b.a(), b.b(), b.c(), b.d()))
            }

            /// set_bone_rotation(i, deg)：局部旋转（度；下次
            /// update_world_transform 生效）。
            #[pymethod]
            fn set_bone_rotation(
                &self,
                i: usize,
                deg: ArgIntoFloat,
                vm: &VirtualMachine,
            ) -> PyResult<()> {
                let mut sk = self.inner.borrow_mut();
                match sk.bone_mut(i) {
                    Some(b) => {
                        b.rotation = deg.into_float() as f32;
                        Ok(())
                    }
                    None => Err(vm.new_value_error(format!("骨骼索引越界: {i}"))),
                }
            }

            /// set_position(x, y)：骨架整体平移。
            #[pymethod]
            fn set_position(&self, x: ArgIntoFloat, y: ArgIntoFloat) {
                let mut sk = self.inner.borrow_mut();
                sk.x = x.into_float() as f32;
                sk.y = y.into_float() as f32;
            }

            /// set_scale(sx, sy)：骨架整体缩放（负值翻转）。
            #[pymethod]
            fn set_scale(&self, sx: ArgIntoFloat, sy: ArgIntoFloat) {
                let mut sk = self.inner.borrow_mut();
                sk.scale_x = sx.into_float() as f32;
                sk.scale_y = sy.into_float() as f32;
            }

            /// render_commands() -> list：渲染命令流（每条 =
            /// (page, r, g, b, a, positions 扁平 xy, uvs 扁平 uv,
            /// triangles 索引)——引擎带贴图 mesh draw 面的消费形态；
            /// v1 数据出口）。
            #[pymethod]
            fn render_commands(&self, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
                let sk = self.inner.borrow();
                let cmds = sp::render::render(&sk);
                let items: Vec<PyObjectRef> = cmds
                    .iter()
                    .map(|c| {
                        let pos: Vec<PyObjectRef> = c
                            .positions
                            .iter()
                            .flat_map(|p| [p.x, p.y])
                            .map(|v| vm.ctx.new_float(v as f64).into())
                            .collect();
                        let uvs: Vec<PyObjectRef> = c
                            .uvs
                            .iter()
                            .map(|v| vm.ctx.new_float(*v as f64).into())
                            .collect();
                        let tris: Vec<PyObjectRef> = c
                            .triangles
                            .iter()
                            .map(|t| vm.ctx.new_int(*t as i64).into())
                            .collect();
                        vm.new_tuple((
                            c.page,
                            (c.color.r as f64, c.color.g as f64, c.color.b as f64, c.color.a as f64),
                            PyObjectRef::from(vm.ctx.new_list(pos)),
                            PyObjectRef::from(vm.ctx.new_list(uvs)),
                            PyObjectRef::from(vm.ctx.new_list(tris)),
                        ))
                        .into()
                    })
                    .collect();
                Ok(vm.ctx.new_list(items).into())
            }
        }

        #[pyclass(with(Constructor))]
        impl PySpineState {
            /// set_animation(anim, looping)：track 0 设当前动画（环形/单发
            /// 由 looping 定）。
            #[pymethod]
            fn set_animation(
                &self,
                anim: PyRef<PySpineAnimation>,
                looping: Option<bool>,
                _vm: &VirtualMachine,
            ) {
                self.inner
                    .borrow_mut()
                    .set_animation(anim.0.clone(), looping.unwrap_or(true));
            }

            /// update(dt)：推进时间轴（帧循环调用；dt = 秒）。
            #[pymethod]
            fn update(&self, dt: ArgIntoFloat) {
                self.inner.borrow_mut().update(dt.into_float() as f32);
            }

            /// apply(skeleton)：把当前时间轴姿态写进骨架（写后调
            /// skeleton.update_world_transform() 再查询/渲染）。
            #[pymethod]
            fn apply(&self, skeleton: PyRef<PySpineSkeleton>) {
                let mut sk = skeleton.inner.borrow_mut();
                self.inner.borrow_mut().apply(&mut sk);
            }
        }

        /// demo_textured_skeleton() -> SkeletonData：带贴图 region 的
        /// 程序化骨架（三骨链 + body/hand 两方块 slot + default skin
        /// ——零资产渲染 demo 的数据源，page0 纹理由调用方供给）。
        #[pyfunction]
        fn demo_textured_skeleton(vm: &VirtualMachine) -> PyRef<PySpineSkeletonData> {
            PySpineSkeletonData {
                data: RefCell::new(sp::demo_textured_skeleton()),
            }
            .into_ref(&vm.ctx)
        }
        /// load_with_atlas(json_text, atlas_text) -> (SkeletonData, 页数)：
        /// JSON 骨架 + atlas 元数据一体加载并绑定（crate 侧完成 page/uv
        /// 重映射——Atlas 类型不外露）。
        #[pyfunction]
        fn load_with_atlas(
            json_text: PyStrRef,
            atlas_text: PyStrRef,
            vm: &VirtualMachine,
        ) -> PyResult<(PyRef<PySpineSkeletonData>, usize)> {
            let j = json_text
                .to_str()
                .ok_or_else(|| vm.new_value_error("文本含非 UTF-8 字符"))?;
            let a = atlas_text
                .to_str()
                .ok_or_else(|| vm.new_value_error("文本含非 UTF-8 字符"))?;
            match sp::load_with_atlas(j, a) {
                Ok((data, pages)) => Ok((
                    PySpineSkeletonData {
                        data: RefCell::new(data),
                    }
                    .into_ref(&vm.ctx),
                    pages,
                )),
                Err(e) => Err(vm.new_value_error(format!("骨架+atlas 加载失败: {e:?}"))),
            }
        }
    }

pub(crate) use self::pygame_spine::module_def;
pub(crate) use self::pygame_spine::PySpineSkeleton;
