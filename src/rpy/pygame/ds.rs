//! pygame.ds 绑定（**stdlite 定约 2026-11-24**：数据结构层——
//! starfish-ds 直通；**值载荷索引定约**：Rust 侧只承载 (f64 优先级,
//! usize 载荷索引) 配对——py 值载荷由绑定层 Vec 侧表持有，零对象
//! 编组；A* 主场景 = f32 cost 直算不回 py `__lt__`）。
//!
//! 类结构定约（random.rs 正典）：类声明 `#[pyclass]` + `impl
//! Constructor`（无标注）+ 方法 impl 块 `#[pyclass(with(Constructor))]`；
//! 槽禁令：`__len__`/`__init__` 禁作 pymethod（AsSequence/Initializer
//! 冲突）——普通方法名 len()/构造收参。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use crate::rpy::{bridge, args};
use starfish_ds as sds;
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    builtins::PyType,
    types::Constructor,
};
use std::cell::RefCell;

#[pymodule]
    pub mod ds {
        use rustpython_derive::{pyclass, PyPayload, FromArgs};
        use rustpython_vm::types::Constructor;
        use rustpython_vm::convert::TryFromObject;
        use super::*;

        /// Heap(capacity) 构造收参（容量为预估提示——堆体即刻可用）
        #[derive(FromArgs)]
        pub(crate) struct HeapArgs {
            #[pyarg(any, optional)]
            capacity: Option<i64>,
        }

        // ── Heap：二叉最小堆（f64 优先级 + py 值载荷侧表）──
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame.ds", name = "Heap", unhashable = true)]
        pub(crate) struct PyHeap {
            heap: RefCell<sds::Heap>,
            values: RefCell<Vec<PyObjectRef>>,
        }

        impl std::fmt::Debug for PyHeap {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "Heap(len={})", self.heap.borrow().len())
            }
        }

        impl Constructor for PyHeap {
            type Args = HeapArgs;

            fn py_new(
                _cls: &Py<PyType>,
                _args: HeapArgs,
                _vm: &VirtualMachine,
            ) -> PyResult<Self> {
                Ok(Self {
                    heap: RefCell::new(sds::Heap::new()),
                    values: RefCell::new(Vec::new()),
                })
            }
        }

        #[pyclass(with(Constructor))]
        impl PyHeap {
            /// push(priority, value)：入堆（值载荷存侧表，堆体只持索引）
            #[pymethod]
            fn push(&self, priority: f64, value: PyObjectRef) {
                let idx = self.values.borrow().len();
                self.values.borrow_mut().push(value);
                self.heap.borrow_mut().push(priority, idx);
            }

            /// pop() -> (priority, value) | None：最小 priority 项出堆
            #[pymethod]
            fn pop(&self) -> Option<(f64, PyObjectRef)> {
                self.heap.borrow_mut().pop().map(|(p, idx)| {
                    let v = self.values.borrow_mut()[idx].clone();
                    (p, v)
                })
            }

            /// push_pop(priority, value) -> (priority, value)：
            /// 入堆即出（A* 高频组合——返回被顶掉的旧项或自身）
            #[pymethod]
            fn push_pop(&self, priority: f64, value: PyObjectRef) -> (f64, PyObjectRef) {
                let idx = self.values.borrow().len();
                self.values.borrow_mut().push(value);
                let (p, i) = self.heap.borrow_mut().push_pop(priority, idx);
                let v = self.values.borrow_mut()[i].clone();
                (p, v)
            }

            /// from_pairs(pairs)：批量建堆（Floyd 线性下滤——
            /// [(priority, value)] 列表一次构造）
            #[pymethod]
            fn from_pairs(&self, pairs: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
                let len = pairs.length(vm)?.max(0);
                let mut items = Vec::with_capacity(len as usize);
                for i in 0..len {
                    let item = pairs.get_item(&i, vm)?;
                    let p = f64::try_from_object(vm, item.get_item(&0, vm)?)?;
                    let v = item.get_item(&1, vm)?;
                    items.push((p, items.len()));
                    self.values.borrow_mut().push(v);
                }
                *self.heap.borrow_mut() = sds::Heap::from_pairs(items);
                Ok(())
            }

            /// len() -> int：堆内项数
            #[pymethod]
            fn len(&self) -> i64 {
                self.heap.borrow().len() as i64
            }

            /// is_empty() -> bool
            #[pymethod]
            fn is_empty(&self) -> bool {
                self.heap.borrow().is_empty()
            }
        }

        // ── SortedList：有序数组（二分插入——bisect 等价）──
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame.ds", name = "SortedList", unhashable = true)]
        pub(crate) struct PySortedList {
            list: RefCell<sds::SortedList>,
            values: RefCell<Vec<PyObjectRef>>,
        }

        impl std::fmt::Debug for PySortedList {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "SortedList(len={})", self.list.borrow().len())
            }
        }

        impl Constructor for PySortedList {
            type Args = ();

            fn py_new(
                _cls: &Py<PyType>,
                _args: (),
                _vm: &VirtualMachine,
            ) -> PyResult<Self> {
                Ok(Self {
                    list: RefCell::new(sds::SortedList::new()),
                    values: RefCell::new(Vec::new()),
                })
            }
        }

        #[pyclass(with(Constructor))]
        impl PySortedList {
            /// insert(key, value)：有序插入（二分定位）
            #[pymethod]
            fn insert(&self, key: f64, value: PyObjectRef) {
                let idx = self.values.borrow().len();
                self.values.borrow_mut().push(value);
                self.list.borrow_mut().insert(key, idx);
            }

            /// get(index) -> (key, value) | None：有序第 i 项
            #[pymethod]
            fn get(&self, index: i64) -> Option<(f64, PyObjectRef)> {
                let idx = index.max(0) as usize;
                self.list.borrow().get(idx).map(|(k, i)| {
                    let v = self.values.borrow()[i].clone();
                    (k, v)
                })
            }

            /// first() -> (key, value) | None：最小键项
            #[pymethod]
            fn first(&self) -> Option<(f64, PyObjectRef)> {
                self.list.borrow().first().map(|(k, i)| {
                    let v = self.values.borrow()[i].clone();
                    (k, v)
                })
            }

            /// last() -> (key, value) | None：最大键项
            #[pymethod]
            fn last(&self) -> Option<(f64, PyObjectRef)> {
                self.list.borrow().last().map(|(k, i)| {
                    let v = self.values.borrow()[i].clone();
                    (k, v)
                })
            }

            /// bisect_left(key) -> int：左插入位
            #[pymethod]
            fn bisect_left(&self, key: f64) -> i64 {
                self.list.borrow().bisect_left(key) as i64
            }

            /// bisect_right(key) -> int：右插入位
            #[pymethod]
            fn bisect_right(&self, key: f64) -> i64 {
                self.list.borrow().bisect_right(key) as i64
            }

            /// contains(key) -> bool：键存在性（O(log)）
            #[pymethod]
            fn contains(&self, key: f64) -> bool {
                self.list.borrow().contains(key)
            }

            /// remove_at(index)：移除有序第 i 项
            #[pymethod]
            fn remove_at(&self, index: i64) {
                let idx = index.max(0) as usize;
                let _ = self.list.borrow_mut().remove_at(idx);
            }

            /// range(lo, hi) -> list：键 ∈ [lo, hi) 的值列表
            #[pymethod]
            fn range(&self, lo: f64, hi: f64, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
                let items: Vec<PyObjectRef> = self
                    .list
                    .borrow()
                    .range(lo, hi)
                    .into_iter()
                    .map(|i| self.values.borrow()[i].clone())
                    .collect();
                Ok(vm.ctx.new_list(items).into())
            }

            /// len() -> int：项数
            #[pymethod]
            fn len(&self) -> i64 {
                self.list.borrow().len() as i64
            }

            /// is_empty() -> bool
            #[pymethod]
            fn is_empty(&self) -> bool {
                self.list.borrow().is_empty()
            }
        }

        // ── UnionFind：并查集（路径压缩 + 按秩合并）──
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame.ds", name = "UnionFind", unhashable = true)]
        pub(crate) struct PyUnionFind {
            uf: RefCell<sds::UnionFind>,
        }

        impl std::fmt::Debug for PyUnionFind {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "UnionFind(n={})", self.uf.borrow().len())
            }
        }

        impl Constructor for PyUnionFind {
            type Args = i64;

            fn py_new(
                _cls: &Py<PyType>,
                n: i64,
                _vm: &VirtualMachine,
            ) -> PyResult<Self> {
                Ok(Self {
                    uf: RefCell::new(sds::UnionFind::new(n.max(0) as usize)),
                })
            }
        }

        #[pyclass(with(Constructor))]
        impl PyUnionFind {
            /// find(a) -> int：根（路径压缩）
            #[pymethod]
            fn find(&self, a: i64) -> i64 {
                self.uf.borrow_mut().find(a.max(0) as usize) as i64
            }

            /// union(a, b) -> bool：合并（不同树才合并 = True）
            #[pymethod]
            fn union(&self, a: i64, b: i64) -> bool {
                self.uf.borrow_mut().union(a.max(0) as usize, b.max(0) as usize)
            }

            /// connected(a, b) -> bool：连通判定
            #[pymethod]
            fn connected(&self, a: i64, b: i64) -> bool {
                self.uf.borrow_mut().connected(a.max(0) as usize, b.max(0) as usize)
            }

            /// components() -> int：当前连通分量数
            #[pymethod]
            fn components(&self) -> i64 {
                self.uf.borrow().components() as i64
            }

            /// len() -> int：元素总数
            #[pymethod]
            fn len(&self) -> i64 {
                self.uf.borrow().len() as i64
            }
        }
    }

pub(crate) use ds::module_def;
