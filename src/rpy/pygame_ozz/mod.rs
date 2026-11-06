//! pygame_ozz —— 3D 骨骼动画绑定（阶段 4；独立顶层 Python 模块
//! ——pygame_spine 同范式：`import pygame_ozz` 直达）。
//!
//! 上游 = ozz-animation-rs（MPL-2.0，ozz-animation 的 Rust 移植——
//! 确定性采样 / 混合 / IK / 蒙皮 job 管线），经 pygame_ozz 兄弟 crate
//! 的 Player 会话（采样→局部→模型矩阵三段 job 薄封装）。
//!
//! ⚠️ 工具链：上游使用 std::simd——**需 nightly rustc 构建**。feature
//! `ozz` 门控（不在 default——stable 默认构建不含本模块，import 即
//! AttributeError）；nightly 宿主构建激活
//! （`cargo +nightly build --features ozz`）。
//!
//! 数据流：`demo_player()` / `Player(skeleton_bytes, animation_bytes)`
//! → `sample(ratio)` → `model_matrices()`（每关节 16 f32 列主矩阵流
//! ——引擎蒙皮 mesh draw 面落地后接 GPU；v1 数据出口）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    types::Constructor, builtins::PyType,
};
use std::cell::RefCell;

#[pymodule(name = "pygame_ozz")]
pub mod pygame_ozz {
    use rustpython_derive::{pyclass, FromArgs};
    use rustpython_vm::{
        function::ArgIntoFloat, PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
        types::Constructor, builtins::PyType,
    };
    use std::cell::RefCell;

    // 内层模块与 crate 同名——`::` 全路径别名（pygame_spine 同款）
    use ::pygame_ozz as ozz;

    /// OzzPlayer（采样→模型矩阵会话——sample 就地缓存，model_matrices
    /// 读缓存；nightly 构建专属）。
    #[derive(PyPayload)]
    #[pyattr]
    #[pyclass(module = "pygame_ozz", name = "OzzPlayer", unhashable = true)]
    pub(crate) struct PyOzzPlayer {
        pub(crate) inner: RefCell<ozz::Player>,
    }

    impl std::fmt::Debug for PyOzzPlayer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&format!(
                "OzzPlayer(<{} joints>)",
                self.inner.borrow().num_joints()
            ))
        }
    }

    /// 构造收参（双 ozz archive 字节流——skeleton.ozz + animation.ozz）。
    #[derive(FromArgs)]
    pub(crate) struct PlayerArgs {
        skeleton_bytes: Vec<u8>,
        animation_bytes: Vec<u8>,
    }

    impl Constructor for PyOzzPlayer {
        type Args = PlayerArgs;

        fn py_new(
            _cls: &Py<PyType>,
            args: Self::Args,
            vm: &VirtualMachine,
        ) -> PyResult<Self> {
            match ozz::Player::new(&args.skeleton_bytes, &args.animation_bytes) {
                Ok(p) => Ok(Self {
                    inner: RefCell::new(p),
                }),
                Err(e) => Err(vm.new_value_error(format!("ozz 资产加载失败: {e:?}"))),
            }
        }
    }

    #[pyclass(with(Constructor))]
    impl PyOzzPlayer {
        /// num_joints() -> int：关节数。
        #[pymethod]
        fn num_joints(&self) -> usize {
            self.inner.borrow().num_joints()
        }

        /// joint_names() -> list[str]：关节名表（骨序）。
        #[pymethod]
        fn joint_names(&self, vm: &VirtualMachine) -> PyObjectRef {
            let items: Vec<PyObjectRef> = self
                .inner
                .borrow()
                .joint_names()
                .into_iter()
                .map(|s| vm.ctx.new_str(s).into())
                .collect();
            PyObjectRef::from(vm.ctx.new_list(items))
        }

        /// duration() -> float：动画时长（秒）。
        #[pymethod]
        fn duration(&self) -> f32 {
            self.inner.borrow().duration()
        }

        /// sample(ratio)：采样（0..1 归一动画时间）→ 模型矩阵重建。
        #[pymethod]
        fn sample(&self, ratio: ArgIntoFloat, vm: &VirtualMachine) -> PyResult<()> {
            self.inner
                .borrow_mut()
                .sample(ratio.into_float() as f32)
                .map_err(|e| vm.new_value_error(format!("采样失败: {e:?}")))
        }

        /// model_matrices() -> list[float]：模型矩阵流（每关节 16 f32
        /// 列主——蒙皮 mesh draw 面的消费形态；v1 数据出口）。
        #[pymethod]
        fn model_matrices(&self, vm: &VirtualMachine) -> PyObjectRef {
            let items: Vec<PyObjectRef> = self
                .inner
                .borrow()
                .model_matrices()
                .into_iter()
                .map(|v| vm.ctx.new_float(v as f64).into())
                .collect();
            PyObjectRef::from(vm.ctx.new_list(items))
        }

        /// joint_positions_xy() -> list[(x, y)]：关节世界位（上一次
        /// sample 的缓存——骨骼可视化 / 平面投影的消费形态）。
        #[pymethod]
        fn joint_positions_xy(&self, vm: &VirtualMachine) -> PyObjectRef {
            let items: Vec<PyObjectRef> = self
                .inner
                .borrow()
                .joint_positions_xy()
                .into_iter()
                .map(|(x, y)| vm.new_tuple((x as f64, y as f64)).into())
                .collect();
            PyObjectRef::from(vm.ctx.new_list(items))
        }

        /// joint_parents() -> list[int]：父关节索引表（骨序；根 = -1）。
        #[pymethod]
        fn joint_parents(&self, vm: &VirtualMachine) -> PyObjectRef {
            let items: Vec<PyObjectRef> = self
                .inner
                .borrow()
                .joint_parents()
                .into_iter()
                .map(|p| vm.ctx.new_int(p as i64).into())
                .collect();
            PyObjectRef::from(vm.ctx.new_list(items))
        }
    }

    /// demo_player() -> OzzPlayer：playback 测试资产直构（零资产测试
    /// 链——绑定/测试即装即跑）。
    #[pyfunction]
    fn demo_player(vm: &VirtualMachine) -> PyResult<PyRef<PyOzzPlayer>> {
        match ozz::Player::new(ozz::demo_skeleton_bytes(), ozz::demo_animation_bytes()) {
            Ok(p) => Ok(PyOzzPlayer {
                inner: RefCell::new(p),
            }
            .into_ref(&vm.ctx)),
            Err(e) => Err(vm.new_value_error(format!("ozz demo 资产加载失败: {e:?}"))),
        }
    }
}

pub(crate) use self::pygame_ozz::module_def;
