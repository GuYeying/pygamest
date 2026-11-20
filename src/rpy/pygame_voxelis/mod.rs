//! pygame_voxelis —— 稀疏体素八叉树绑定（独立顶层 Python 模块：
//! `import pygame_voxelis` 直达；pygame_spine/pygame_ozz 同范式）。
//!
//! 上游 = voxelis crate（25.4.0，MPL/Apache 双许可）——VoxModel 分块
//! 体素树：chunk（VoxChunk，八叉树 VoxTree 承载）按 HashMap 稀疏分布，
//! 值经 VoxInterner<i32> 去重存储（air = 0——`voxel != T::default()`
//! 分支证明 set(pos, 0) 即删除并 prune 空节点）。
//!
//! 绑定面 = MC 玩法所需最小集（PyVoxWorld 包装 VoxModel）：
//! - **全局体素坐标**直接编辑：set_voxel/remove_voxel/get_voxel——
//!   chunk 定位（div_euclid）+ chunk 内局部坐标（rem_euclid）在绑定
//!   层封装，Python 零 octree 概念
//! - **渲染数组出口**：generate_mesh() → (vertices, normals, indices)
//!   平面化 f32/u32 列表（全局坐标——chunk.get_world_position() 做
//!   offset），接 pygame.render mesh/pipeline 即渲染；面剔除不在上游
//!   能力内（tree 遍历跳空块），案例级规模直接可用
//! - 物理/射线照旧走 pygame_physics 的 physics3d 面（cuboid per 体素
//!   + cast_ray），本模块只供数不包渲染/物理
//!
//! 坐标定约（上游语义）：体素格 = chunk 局域 [0, 2^max_depth) 均分；
//! 体素世界尺寸 vs = chunk_world_size / 2^max_depth；mesh 顶点 = 格
//! 中心 ± 半体素（上游 0.5 offset 习语）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    types::Constructor, builtins::PyType,
};
use std::cell::RefCell;
use std::sync::Arc;

pub(crate) use self::pygame_voxelis::module_def;

#[pymodule(name = "pygame_voxelis")]
pub mod pygame_voxelis {
    use glam_v29::{IVec3, Vec3};
    use rustpython_derive::{pyclass, FromArgs};
    use rustpython_vm::{
        PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
        types::Constructor, builtins::PyType,
    };
    use parking_lot_v12::{RwLock, RwLockReadGuard, RwLockWriteGuard};
    use std::cell::RefCell;
    use std::sync::Arc;

    use ::voxelis::spatial::{VoxOpsRead, VoxOpsState, VoxOpsWrite};
    use ::voxelis::world::VoxModel;
    use ::voxelis::{Lod, MaxDepth};

    /// 体素世界会话（VoxModel 薄壳——稀疏分块体素树 + interner 去重）。
    ///
    /// max_depth = 每 chunk 八叉树深度（每轴体素数 = 2^max_depth，默认
    /// 4 = 16×16×16/chunk）；chunk_world_size = chunk 世界边长（体素
    /// 世界尺寸 = chunk_world_size / 2^max_depth，默认 16 → 体素 1.0）。
    /// chunk 按需创建（HashMap 稀疏——写哪建哪，clear 后归零）。
    #[derive(PyPayload)]
    #[pyattr]
    #[pyclass(module = "pygame_voxelis", name = "VoxWorld", unhashable = true)]
    pub(crate) struct PyVoxWorld {
        pub(crate) model: RefCell<VoxModel>,
        /// interner Arc（构造时从 model clone 持有——共享同一数据，编辑
        /// 路径 write / 读路径 read；免每次调用再 clone）
        pub(crate) interner: Arc<RwLock<::voxelis::VoxInterner<i32>>>,
        pub(crate) max_depth: u8,
        pub(crate) chunk_world_size: f32,
    }

    impl std::fmt::Debug for PyVoxWorld {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let chunks = self.model.borrow().chunks.len();
            f.write_str(&format!("VoxWorld(<{chunks} chunks>)"))
        }
    }

    /// 构造参（三可选——max_depth / chunk_world_size / memory_budget；
    /// `default` 标注 = 未传走 Option::default = None）
    #[derive(FromArgs)]
    pub(crate) struct VoxWorldArgs {
        #[pyarg(any, default)]
        pub(crate) max_depth: Option<u8>,
        #[pyarg(any, default)]
        pub(crate) chunk_world_size: Option<f32>,
        #[pyarg(any, default)]
        pub(crate) memory_budget: Option<usize>,
    }

    #[pyclass(with(Constructor))]
    impl PyVoxWorld {
        /// set_voxel(x, y, z, value)——value=0 即删除（air 语义，上游
        /// prune 空节点）；非 0 值 = 放置/改色（interner 去重存储）。
        #[pymethod]
        fn set_voxel(
            &self,
            x: i32,
            y: i32,
            z: i32,
            value: i32,
            _vm: &VirtualMachine,
        ) -> PyResult<()> {
            let vpa = 1i32 << self.max_depth;
            let (cp, lp) = Self::locate(x, y, z, vpa);
            let mut model = self.model.borrow_mut();
            let mut guard = Self::interner_write(&self.interner);
            if value == 0 {
                // 空不建 chunk——remove 打到无 chunk 区域是 no-op
                if model.chunks.contains_key(&cp) {
                    let chunk = model.get_or_create_chunk(cp);
                    chunk.set(&mut guard, lp, 0);
                }
            } else {
                let chunk = model.get_or_create_chunk(cp);
                chunk.set(&mut guard, lp, value);
            }
            Ok(())
        }

        /// remove_voxel(x, y, z)——set_voxel(…, 0) 别名（MC 语义显式面）。
        #[pymethod]
        fn remove_voxel(
            &self,
            x: i32,
            y: i32,
            z: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            self.set_voxel(x, y, z, 0, vm)
        }

        /// 查询体素值（空/air 均回 None——interner 里 0 不入存）。
        #[pymethod]
        fn get_voxel(
            &self,
            x: i32,
            y: i32,
            z: i32,
            _vm: &VirtualMachine,
        ) -> PyResult<Option<i32>> {
            let vpa = 1i32 << self.max_depth;
            let (cp, lp) = Self::locate(x, y, z, vpa);
            let model = self.model.borrow();
            let guard = Self::interner_read(&self.interner);
            Ok(model
                .chunks
                .get(&cp)
                .and_then(|c| c.get(&guard, lp))
                .filter(|v| *v != 0))
        }

        /// 渲染数组出口 (vertices, normals, indices)——三个 float/int
        /// 列表，全局坐标（含 chunk 世界 offset）；空 chunk 跳过，LOD
        /// 恒 0。布局 ["float32x3", "float32x3"] + 索引面接 pygame.render。
        #[pymethod]
        fn generate_mesh(&self, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let model = self.model.borrow();
            let guard = Self::interner_read(&self.interner);
            let mut vertices: Vec<Vec3> = Vec::new();
            let mut normals: Vec<Vec3> = Vec::new();
            let mut indices: Vec<u32> = Vec::new();
            for chunk in model.chunks.values() {
                if chunk.is_empty() {
                    continue;
                }
                chunk.generate_mesh_arrays(
                    &guard,
                    &mut vertices,
                    &mut normals,
                    &mut indices,
                    chunk.get_world_position(),
                    Lod::new(0),
                );
            }
            // Vec3 → 平面化 f32（mesh_builder 收 [f32] 流）——py 面手工
            // 转 list（ToPyObject 无泛型 Vec impl）
            let lv: Vec<PyObjectRef> = vertices
                .iter()
                .flat_map(|v| [v.x, v.y, v.z])
                .map(|f| vm.ctx.new_float(f as f64).into())
                .collect();
            let ln: Vec<PyObjectRef> = normals
                .iter()
                .flat_map(|v| [v.x, v.y, v.z])
                .map(|f| vm.ctx.new_float(f as f64).into())
                .collect();
            let li: Vec<PyObjectRef> = indices
                .iter()
                .map(|&i| vm.ctx.new_int(i as i64).into())
                .collect();
            let t: Vec<PyObjectRef> = vec![
                vm.ctx.new_list(lv).into(),
                vm.ctx.new_list(ln).into(),
                vm.ctx.new_list(li).into(),
            ];
            Ok(vm.ctx.new_tuple(t).into())
        }

        /// generate_mesh_flat() -> (flat_vn, indices)：**唯一顶点
        /// pos3+nrm3 交错平铺 + u16 索引面**——配 ac.mesh(layout, verts,
        /// indices) / ac.update_mesh 原地更新（draw 按索引计数 = 可变
        /// 长度）。py 侧零循环（曾 py 展开 10200 次 ≈ 编辑 remesh 卡帧
        /// 主源之一，2026-11-20 分段计时 remesh=140ms 实锤）
        #[pymethod]
        fn generate_mesh_flat(&self, vm: &VirtualMachine) -> PyResult<(PyObjectRef, PyObjectRef)> {
            let model = self.model.borrow();
            let guard = Self::interner_read(&self.interner);
            let mut vertices: Vec<Vec3> = Vec::new();
            let mut normals: Vec<Vec3> = Vec::new();
            let mut indices: Vec<u32> = Vec::new();
            for chunk in model.chunks.values() {
                if chunk.is_empty() {
                    continue;
                }
                chunk.generate_mesh_arrays(
                    &guard,
                    &mut vertices,
                    &mut normals,
                    &mut indices,
                    chunk.get_world_position(),
                    Lod::new(0),
                );
            }
            let flat: Vec<PyObjectRef> = vertices
                .iter()
                .zip(normals.iter())
                .flat_map(|(v, n)| [v.x, v.y, v.z, n.x, n.y, n.z])
                .map(|f| vm.ctx.new_float(f as f64).into())
                .collect();
            let li: Vec<PyObjectRef> = indices
                .iter()
                .map(|&i| vm.ctx.new_int(i as i64).into())
                .collect();
            Ok((vm.ctx.new_list(flat).into(), vm.ctx.new_list(li).into()))
        }

        /// generate_mesh_bytes() -> (vertex_bytes, index_bytes)：**f32 LE
        /// / u16 LE 原始字节出口**——RP 零逐元素转换（配 mesh_builder
        /// vertices_bytes/index_bytes 与 update_mesh_vertex_bytes/
        /// index_bytes 原地更新；动态 mesh 编辑零转换通路，2026-11-20
        /// ——list 口的 20400 次 new_float ≈ remesh 卡帧残余）
        #[pymethod]
        fn generate_mesh_bytes(&self, vm: &VirtualMachine) -> PyResult<(PyObjectRef, PyObjectRef)> {
            let model = self.model.borrow();
            let guard = Self::interner_read(&self.interner);
            let mut vertices: Vec<Vec3> = Vec::new();
            let mut normals: Vec<Vec3> = Vec::new();
            let mut indices: Vec<u32> = Vec::new();
            for chunk in model.chunks.values() {
                if chunk.is_empty() {
                    continue;
                }
                chunk.generate_mesh_arrays(
                    &guard,
                    &mut vertices,
                    &mut normals,
                    &mut indices,
                    chunk.get_world_position(),
                    Lod::new(0),
                );
            }
            let mut f32s: Vec<f32> = Vec::with_capacity(vertices.len() * 6);
            for (v, n) in vertices.iter().zip(normals.iter()) {
                f32s.extend_from_slice(&[v.x, v.y, v.z, n.x, n.y, n.z]);
            }
            let vbytes =
                unsafe { std::slice::from_raw_parts(f32s.as_ptr() as *const u8, f32s.len() * 4) }
                    .to_vec();
            let ibytes: Vec<u8> = indices
                .iter()
                .flat_map(|&i| (i as u16).to_le_bytes())
                .collect();
            Ok((vm.ctx.new_bytes(vbytes).into(), vm.ctx.new_bytes(ibytes).into()))
        }

        /// 全清 chunk 表（interner 项引用计数不回退——上游无 interner
        /// 清面，案例级内存可接受；长会话建议重建 VoxWorld 代替 clear）。
        #[pymethod]
        fn clear(&self) {
            self.model.borrow_mut().clear();
        }

        /// 体素世界尺寸 = chunk_world_size / 2^max_depth（物理 collider
        /// half extent 与命中点→体素坐标换算的基准）。
        #[pymethod]
        fn voxel_size(&self) -> f32 {
            self.chunk_world_size / (1u32 << self.max_depth) as f32
        }

        /// 非空 chunk 数（稀疏分布监控——案例/调试打点用）。
        #[pymethod]
        fn chunk_count(&self) -> usize {
            self.model.borrow().chunks.len()
        }
    }

    impl PyVoxWorld {
        /// 全局体素坐标 →（chunk 坐标，chunk 内局部坐标）——div/rem_euclid
        /// 支持负坐标（MC 无限平面语义）。
        #[inline]
        fn locate(x: i32, y: i32, z: i32, vpa: i32) -> (IVec3, IVec3) {
            (
                IVec3::new(x.div_euclid(vpa), y.div_euclid(vpa), z.div_euclid(vpa)),
                IVec3::new(x.rem_euclid(vpa), y.rem_euclid(vpa), z.rem_euclid(vpa)),
            )
        }

        #[inline]
        fn interner_write(
            interner: &Arc<RwLock<::voxelis::VoxInterner<i32>>>,
        ) -> RwLockWriteGuard<'_, ::voxelis::VoxInterner<i32>> {
            interner.write()
        }

        #[inline]
        fn interner_read(
            interner: &Arc<RwLock<::voxelis::VoxInterner<i32>>>,
        ) -> RwLockReadGuard<'_, ::voxelis::VoxInterner<i32>> {
            interner.read()
        }
    }

    impl Constructor for PyVoxWorld {
        type Args = VoxWorldArgs;

        fn py_new(
            _cls: &Py<PyType>,
            args: Self::Args,
            vm: &VirtualMachine,
        ) -> PyResult<Self> {
            let max_depth = args.max_depth.unwrap_or(4);
            let chunk_world_size = args.chunk_world_size.unwrap_or(16.0);
            let memory_budget = args.memory_budget.unwrap_or(16 * 1024 * 1024);
            let max = MaxDepth::try_from(max_depth)
                .map_err(|e| vm.new_value_error(format!("max_depth={max_depth} 超限: {e}")))?;
            if chunk_world_size <= 0.0 {
                return Err(
                    vm.new_value_error(format!("chunk_world_size 须 > 0，得 {chunk_world_size}"))
                );
            }
            let model = VoxModel::new(max, chunk_world_size, memory_budget);
            let interner = model.get_interner();
            Ok(PyVoxWorld {
                model: RefCell::new(model),
                interner,
                max_depth,
                chunk_world_size,
            })
        }
    }
}
