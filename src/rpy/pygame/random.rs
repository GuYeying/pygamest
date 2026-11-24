//! pygame.random 绑定（**stdlite 定约 2026-11-24**：确定性随机层——
//! starfish-rng PCG32 直通；跨平台同 seed 同序列——联机同步/存档回放/
//! 程序化生成的硬前提，wasm JS 随机不可播种 = 标准库路线死路）。
//!
//! API 面 = 标准兼容子集（seed/random/uniform/randint/choice/shuffle/
//! gaussian）+ 游戏扩展（unit_vec2/3、in_circle/sphere、rand_list 批量口）
//! + Random(seed) 独立流（多系统子流互不踩）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use crate::rpy::{bridge, args};
use starfish_rng as srng;
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    builtins::PyStr,
};

#[pymodule]
    pub mod random {
        use rustpython_derive::{pyclass, PyPayload};
        use rustpython_vm::types::Constructor;
        use rustpython_vm::function::ArgIntoFloat;
        use std::cell::RefCell;
        use rustpython_vm::builtins::PyType;
        use super::srng;
        use super::{VirtualMachine, PyObjectRef, PyResult, Py};

        /// 模块级函数——默认流（thread_local PCG32）
        #[pyfunction]
        fn seed(n: i64, vm: &VirtualMachine) {
            srng::seed(n as u64);
            let _ = vm;
        }

        #[pyfunction]
        fn random() -> f64 {
            srng::random() as f64
        }

        #[pyfunction]
        fn uniform(a: ArgIntoFloat, b: ArgIntoFloat) -> f64 {
            srng::uniform(a.into_float() as f32, b.into_float() as f32) as f64
        }

        #[pyfunction]
        fn randint(a: i64, b: i64) -> i64 {
            srng::randint(a, b)
        }

        /// choice(seq)：序列取一（len 索引——通用序列口）
        #[pyfunction]
        fn choice(seq: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let len = seq
                .length(vm)
                .map_err(|e| vm.new_value_error(crate::rpy::bridge::exc_msg(vm, e)))?;
            let i = srng::choice_len(len.max(0) as usize);
            seq.get_item(&i, vm)
        }

        /// shuffle(seq) -> list：**新列表返回**（v1 定约——in-place 突变
        /// 留 v2 口；Fisher-Yates Rust 直算零 py 编组）
        #[pyfunction]
        fn shuffle(seq: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let len = seq
                .length(vm)
                .map_err(|e| vm.new_value_error(crate::rpy::bridge::exc_msg(vm, e)))?;
            let order = srng::shuffle_order(len.max(0) as usize);
            let mut items = Vec::with_capacity(len.max(0) as usize);
            for i in order {
                items.push(seq.get_item(&i, vm)?);
            }
            Ok(vm.ctx.new_list(items).into())
        }

        #[pyfunction]
        fn gaussian(mu: ArgIntoFloat, sigma: ArgIntoFloat) -> f64 {
            srng::gaussian(mu.into_float() as f32, sigma.into_float() as f32) as f64
        }

        /// unit_vec2() -> (x, y)：均匀 2D 方向
        #[pyfunction]
        fn unit_vec2() -> (f64, f64) {
            let (x, y) = srng::unit_vec2();
            (x as f64, y as f64)
        }

        /// unit_vec3() -> (x, y, z)：均匀 3D 方向（球面均匀）
        #[pyfunction]
        fn unit_vec3() -> (f64, f64, f64) {
            let (x, y, z) = srng::unit_vec3();
            (x as f64, y as f64, z as f64)
        }

        /// in_circle(r) -> (x, y)：圆内均匀点（sqrt 反变换）
        #[pyfunction]
        fn in_circle(r: ArgIntoFloat) -> (f64, f64) {
            let (x, y) = srng::in_circle(r.into_float() as f32);
            (x as f64, y as f64)
        }

        /// in_sphere(r) -> (x, y, z)：球内均匀点（cbrt 反变换）
        #[pyfunction]
        fn in_sphere(r: ArgIntoFloat) -> (f64, f64, f64) {
            let (x, y, z) = srng::in_sphere(r.into_float() as f32);
            (x as f64, y as f64, z as f64)
        }

        /// rand_list(n) -> [f64]：**批量口**（一次边界穿越 n 个）
        #[pyfunction]
        fn rand_list(n: i64, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let n = n.clamp(0, 1_000_000) as usize;
            let vals = srng::rand_list(n);
            let items: Vec<PyObjectRef> = vals
                .iter()
                .map(|v| vm.ctx.new_float(*v as f64).into())
                .collect();
            Ok(vm.ctx.new_list(items).into())
        }

        /// Random(seed)：**独立确定性流**（多系统子流互不踩——地图生成/
        /// AI/视觉噪声各持一个）
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame.random", name = "Random", unhashable = true)]
        pub(crate) struct PyRandom {
            state: RefCell<srng::Pcg32>,
        }

        impl std::fmt::Debug for PyRandom {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "Random(<pcg32>)")
            }
        }

        impl Constructor for PyRandom {
            type Args = i64;

            fn py_new(
                _cls: &Py<PyType>,
                seed: Self::Args,
                _vm: &VirtualMachine,
            ) -> PyResult<Self> {
                Ok(Self {
                    state: RefCell::new(srng::Pcg32::new(seed as u64, 0xDA3E39CB94B95BDB)),
                })
            }
        }

        #[pyclass(with(Constructor))]
        impl PyRandom {
            #[pymethod]
            fn reseed(&self, n: i64) {
                *self.state.borrow_mut() = srng::Pcg32::new(n as u64, 0xDA3E39CB94B95BDB);
            }

            #[pymethod]
            fn random(&self) -> f64 {
                self.state.borrow_mut().next_f32() as f64
            }

            #[pymethod]
            fn uniform(&self, a: ArgIntoFloat, b: ArgIntoFloat) -> f64 {
                self.state
                    .borrow_mut()
                    .uniform(a.into_float() as f32, b.into_float() as f32) as f64
            }

            #[pymethod]
            fn randint(&self, a: i64, b: i64) -> i64 {
                self.state.borrow_mut().randint(a, b)
            }

            #[pymethod]
            fn gaussian(&self, mu: ArgIntoFloat, sigma: ArgIntoFloat) -> f64 {
                self.state
                    .borrow_mut()
                    .gaussian(mu.into_float() as f32, sigma.into_float() as f32) as f64
            }

            /// choice_from(len)：len 索引取一（py 侧 choice(seq) 组装——
            /// 避免实例层重复绑序列口）
            #[pymethod]
            fn choice_from(&self, len: i64) -> i64 {
                self.state.borrow_mut().rand_bounded(len.max(0) as u64) as i64
            }

            #[pymethod]
            fn unit_vec2(&self) -> (f64, f64) {
                let (x, y) = self.state.borrow_mut().unit_vec2();
                (x as f64, y as f64)
            }

            #[pymethod]
            fn unit_vec3(&self) -> (f64, f64, f64) {
                let (x, y, z) = self.state.borrow_mut().unit_vec3();
                (x as f64, y as f64, z as f64)
            }

            #[pymethod]
            fn in_circle(&self, r: ArgIntoFloat) -> (f64, f64) {
                let (x, y) = self.state.borrow_mut().in_circle(r.into_float() as f32);
                (x as f64, y as f64)
            }

            #[pymethod]
            fn in_sphere(&self, r: ArgIntoFloat) -> (f64, f64, f64) {
                let (x, y, z) = self.state.borrow_mut().in_sphere(r.into_float() as f32);
                (x as f64, y as f64, z as f64)
            }

            /// rand_list(n) -> [f64]：批量口
            #[pymethod]
            fn rand_list(&self, n: i64, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
                let n = n.clamp(0, 1_000_000) as usize;
                let vals: Vec<PyObjectRef> = (0..n)
                    .map(|_| self.state.borrow_mut().next_f32() as f64)
                    .map(|v| vm.ctx.new_float(v).into())
                    .collect();
                Ok(vm.ctx.new_list(vals).into())
            }
        }
    }

pub(crate) use random::module_def;
