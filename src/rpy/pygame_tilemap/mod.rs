//! pygame_tilemap —— Tiled 编辑器地图绑定（独立顶层 Python 模块：
//! `import pygame_tilemap` 直达；pygame_voxelis 同范式）。
//!
//! 上游 = tiled crate（0.16.0，MIT）——TMX/TSX 格式标准解析（Tiled
//! Map Editor 行业格式：多层 tile 层 + 图集 tileset + 对象层）。经
//! `ResourceReader` trait 闭包 blanket impl 接 `starfish_pygame::
//! sfx::read_block`——TMX/外部 TSX/图集引用全走密文资源链（dev 宿主
//! 明文透传，runtime 全平台统一），绑定零 fs 直访。
//!
//! 绑定面 = 渲染分层所需最小集（PyTilemap 包装 Map）：
//! - **元数据**：width/height/tile_width/tile_height/layer_count
//! - **层枚举**：layer_info(i)（名/类/可见/不透明度）
//! - **数据出口**：layer_tiles(i)——有限 tile 层全量展开
//!   (x, y, tileset_index, tile_id, flip_h, flip_v, flip_d)（None 跳
//!   过——air 格不入列）
//! - **渲染数组**：layer_mesh(i)——per-tile quad 顺序展开
//!   (verts = 像素域 x/y, uvs = 图集 UV；flip 的 UV 序列变换绑定层
//!   处理)，接 pygame.render mesh ["float32x2","float32x2"] + 图集
//!   纹理即渲染——**每层一 mesh 一 pipeline 按层序 draw = 着色器
//!   渲染层次拓展**（视差/光照/水波等层专属 shader 分层叠加）
//! - **图集面**：tileset_info(i)——图集切分元数据（图路径/尺寸/
//!   瓦片尺寸/列数；图字节 py 侧 pygame.image.load 走 sfx）
//!
//! 坐标定约：层格 (x, y) → 像素 (x*tile_width, y*tile_height)
//! （Tiled y-down 顶格对齐；世界系原点 = 层左上）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    types::Constructor, builtins::PyType,
};
use std::cell::RefCell;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

pub(crate) use self::pygame_tilemap::module_def;

#[pymodule(name = "pygame_tilemap")]
pub mod pygame_tilemap {
    use rustpython_derive::{pyclass, FromArgs};
    use rustpython_vm::{
        PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
        types::Constructor, builtins::PyType,
    };
    use std::cell::RefCell;
    use std::io::Cursor;
    use std::path::Path;
    use std::sync::Arc;

    use ::tiled::{FiniteTileLayer, LayerType, Loader, TileLayer};

    /// sfx 资源链 Reader（TMX/外部 TSX/图集引用统一走 sfx::read_block
    /// ——dev 宿主明文透传，runtime 密文解密，路径 = 资源根锚定相对）。
    struct SfxReader;

    impl ::tiled::ResourceReader for SfxReader {
        type Resource = Cursor<Vec<u8>>;
        type Error = std::io::Error;

        fn read_from(&mut self, path: &Path) -> Result<Self::Resource, Self::Error> {
            let s = path.to_string_lossy().to_string();
            // 双路径 fallback：①原路径（dev 宿主 base_dir = 脚本目录直读）
            // ②资源根前缀（runtime 形态——RES_DIR_NAME = "assets"，wasm 上
            // fetch 相对页面需带 assets/ 前缀；desktop base_dir = exe 同级
            // 同前缀语义）。②为 runtime/wasm 部署形态的资源根读法。
            if let Ok(b) = starfish_pygame::sfx::read_block(&s) {
                return Ok(Cursor::new(b));
            }
            let res_dir = std::option_env!("RES_DIR_NAME").unwrap_or("assets");
            let prefixed = format!("{res_dir}/{s}");
            match starfish_pygame::sfx::read_block(&prefixed) {
                Ok(b) => Ok(Cursor::new(b)),
                Err(e) => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("sfx 读取 {s}（含 {prefixed} fallback）失败: {e}"),
                )),
            }
        }
    }

    /// Tiled 地图会话（tiled::Map 薄壳——多层 tile 层 + 图集）。
    ///
    /// path = TMX 资源路径（sfx 密文链解析——外部 TSX/图集引用同链）。
    #[derive(PyPayload)]
    #[pyattr]
    #[pyclass(module = "pygame_tilemap", name = "Tilemap", unhashable = true)]
    pub(crate) struct PyTilemap {
        pub(crate) map: RefCell<::tiled::Map>,
    }

    impl std::fmt::Debug for PyTilemap {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let layers = self.map.borrow().layers().count();
            f.write_str(&format!("Tilemap(<{layers} layers>)"))
        }
    }

    #[pyclass(with(Constructor))]
    impl PyTilemap {
        /// 地图宽/高（格数）。
        #[pymethod]
        fn width(&self) -> u32 {
            self.map.borrow().width
        }

        #[pymethod]
        fn height(&self) -> u32 {
            self.map.borrow().height
        }

        /// 瓦片像素尺寸（tile_width × tile_height）。
        #[pymethod]
        fn tile_width(&self) -> u32 {
            self.map.borrow().tile_width
        }

        #[pymethod]
        fn tile_height(&self) -> u32 {
            self.map.borrow().tile_height
        }

        /// 层数（含 tile/object/image/group 各类）。
        #[pymethod]
        fn layer_count(&self) -> usize {
            self.map.borrow().layers().count()
        }

        /// layer_info(i) -> (name, kind, visible, opacity)——kind ∈
        /// "tiles"/"objects"/"image"/"group"。
        #[pymethod]
        fn layer_info(
            &self,
            i: usize,
            vm: &VirtualMachine,
        ) -> PyResult<(String, String, bool, f32)> {
            let map = self.map.borrow();
            let layer = map
                .get_layer(i)
                .ok_or_else(|| vm.new_index_error(format!("层 {i} 越界")))?;
            let kind = match layer.layer_type() {
                LayerType::Tiles(_) => "tiles",
                LayerType::Objects(_) => "objects",
                LayerType::Image(_) => "image",
                LayerType::Group(_) => "group",
            };
            Ok((
                layer.name.clone(),
                kind.to_string(),
                layer.visible,
                layer.opacity,
            ))
        }

        /// layer_tiles(i) -> [(x, y, tileset_index, tile_id, fh, fv, fd)]
        /// ——有限 tile 层全量展开（air 格跳过；fh/fv/fd = 三向翻转）。
        /// 无限层抛 ValueError（v1 只做有限层）。
        #[pymethod]
        fn layer_tiles(
            &self,
            i: usize,
            vm: &VirtualMachine,
        ) -> PyResult<PyObjectRef> {
            let map = self.map.borrow();
            let layer = map
                .get_layer(i)
                .ok_or_else(|| vm.new_index_error(format!("层 {i} 越界")))?;
            let tl = match layer.layer_type() {
                LayerType::Tiles(tl) => tl,
                _ => {
                    return Err(vm.new_value_error(format!(
                        "层 {i} 非 tile 层（layer_info 查类）"
                    )));
                }
            };
            let finite = match tl {
                TileLayer::Finite(f) => f,
                TileLayer::Infinite(_) => {
                    return Err(vm.new_value_error("无限层 v1 不支持（finite-only）"));
                }
            };
            let (w, h) = (finite.width(), finite.height());
            let mut out: Vec<PyObjectRef> = Vec::new();
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    if let Some(td) = finite.get_tile_data(x, y) {
                        // 7 元组手转 py list（ToPyObject 无泛型 Vec 面）
                        let tup: PyObjectRef = vm
                            .new_tuple((
                                x,
                                y,
                                td.tileset_index(),
                                td.id(),
                                td.flip_h,
                                td.flip_v,
                                td.flip_d,
                            ))
                            .into();
                        out.push(tup);
                    }
                }
            }
            Ok(vm.ctx.new_list(out).into())
        }

        /// layer_mesh(i) -> (verts, uvs)——per-tile quad 顺序展开渲染
        /// 数组（**每 tile 6 顶点**两三角；verts = 像素域 x/y；uvs =
        /// 图集归一 UV，flip 三向的 UV 序列变换绑定层处理）。布局
        /// ["float32x2", "float32x2"] 直喂 pygame.render.mesh；**空层
        /// 返回空列表**。⚠️ 图集外瓦片（image collection tileset）不
        /// 支持——图集 tileset 专属。
        #[pymethod]
        fn layer_mesh(&self, i: usize, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let map = self.map.borrow();
            let layer = map
                .get_layer(i)
                .ok_or_else(|| vm.new_index_error(format!("层 {i} 越界")))?;
            let tl = match layer.layer_type() {
                LayerType::Tiles(tl) => tl,
                _ => {
                    return Err(vm.new_value_error(format!(
                        "层 {i} 非 tile 层（layer_info 查类）"
                    )));
                }
            };
            let finite = match tl {
                TileLayer::Finite(f) => f,
                TileLayer::Infinite(_) => {
                    return Err(vm.new_value_error("无限层 v1 不支持（finite-only）"));
                }
            };
            let (w, h) = (finite.width(), finite.height());
            let tw = map.tile_width as f32;
            let th = map.tile_height as f32;
            let tilesets = map.tilesets();

            // 图集切分参数缓存（tileset_index → (img_w, img_h, tile_w, tile_h, columns)）
            let mut atlas: Vec<Option<(f32, f32, f32, f32, u32)>> =
                Vec::with_capacity(tilesets.len());
            for ts in tilesets {
                atlas.push(ts.image.as_ref().map(|img| {
                    (
                        img.width as f32,
                        img.height as f32,
                        ts.tile_width as f32,
                        ts.tile_height as f32,
                        ts.columns,
                    )
                }));
            }

            let mut verts: Vec<f32> = Vec::new();
            let mut uvs: Vec<f32> = Vec::new();
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    let Some(td) = finite.get_tile_data(x, y) else {
                        continue;
                    };
                    let Some((iw, ih, cw, ch, cols)) =
                        atlas.get(td.tileset_index()).copied().flatten()
                    else {
                        // image-collection tileset（无图集）——跳过（文档标注）
                        continue;
                    };
                    // 图集 UV（归一；tile_id 行列切分——Tiled 图集行主序）
                    let (col, row) =
                        ((td.id() % cols.max(1)) as f32, (td.id() / cols.max(1)) as f32);
                    let (u0, v0) = (col * cw / iw, row * ch / ih);
                    let (u1, v1) = (u0 + cw / iw, v0 + ch / ih);
                    // flip 三向 → UV 序列变换（Tiled 语义：d = 对角翻转
                    // = h+v+d 组合旋转；四角 (u0,v0)(u1,v0)(u1,v1)(u0,v1) 的
                    // 角序重排——绑定层出序、py 零感知）
                    let mut corners = [
                        [u0, v1],
                        [u1, v1],
                        [u1, v0],
                        [u0, v0],
                    ]; // BL BR TR TL（v-down 图集域）
                    if td.flip_d {
                        corners = [
                            corners[3], corners[1], corners[2], corners[0],
                        ];
                    }
                    if td.flip_v {
                        corners = [
                            corners[1], corners[0], corners[3], corners[2],
                        ];
                    }
                    if td.flip_h {
                        corners = [
                            corners[2], corners[3], corners[0], corners[1],
                        ];
                    }
                    // quad 像素域（y-down 顶格对齐；Tiled 世界原点 = 左上）
                    let (px, py) = (x as f32 * tw, y as f32 * th);
                    // 顺序展开 6 顶点（BL BR TR / BL TR TL——2 三角）
                    let quad_xy = [
                        [px, py + th],
                        [px + tw, py + th],
                        [px + tw, py],
                        [px, py],
                    ];
                    for (vi, ci) in [(0usize, 0usize), (1, 1), (2, 2), (0, 0), (2, 2), (3, 3)] {
                        verts.push(quad_xy[vi][0]);
                        verts.push(quad_xy[vi][1]);
                        uvs.push(corners[ci][0]);
                        uvs.push(corners[ci][1]);
                    }
                }
            }
            // f32 → py float list 手转（ToPyObject 无泛型 Vec 面），
            // 返回 (verts, uvs) 二元组
            let lv: Vec<PyObjectRef> = verts
                .iter()
                .map(|f| vm.ctx.new_float(*f as f64).into())
                .collect();
            let lu: Vec<PyObjectRef> = uvs
                .iter()
                .map(|f| vm.ctx.new_float(*f as f64).into())
                .collect();
            let pair: Vec<PyObjectRef> = vec![
                vm.ctx.new_list(lv).into(),
                vm.ctx.new_list(lu).into(),
            ];
            Ok(vm.ctx.new_tuple(pair).into())
        }

        /// tileset_info(i) -> (name, image_path, img_w, img_h, tile_w,
        /// tile_h, columns)——图集切分元数据（图字节 py 侧
        /// pygame.image.load(image_path) 走 sfx；外部 TSX 引用同链）。
        #[pymethod]
        fn tileset_info(
            &self,
            i: usize,
            vm: &VirtualMachine,
        ) -> PyResult<(String, String, u32, u32, u32, u32, u32)> {
            let map = self.map.borrow();
            let ts = map
                .tilesets()
                .get(i)
                .cloned()
                .ok_or_else(|| vm.new_index_error(format!("tileset {i} 越界")))?;
            let img = ts.image.as_ref().ok_or_else(|| {
                vm.new_value_error("image-collection tileset（无图集）v1 不支持")
            })?;
            Ok((
                ts.name.to_string(),
                img.source.to_string_lossy().to_string(),
                img.width as u32,
                img.height as u32,
                ts.tile_width,
                ts.tile_height,
                ts.columns,
            ))
        }

        /// tileset_count() -> usize。
        #[pymethod]
        fn tileset_count(&self) -> usize {
            self.map.borrow().tilesets().len()
        }
    }

    impl Constructor for PyTilemap {
        type Args = TilemapArgs;

        fn py_new(
            _cls: &Py<PyType>,
            args: Self::Args,
            vm: &VirtualMachine,
        ) -> PyResult<Self> {
            let mut loader = Loader::with_reader(SfxReader);
            let map = loader.load_tmx_map(&args.path).map_err(|e| {
                vm.new_value_error(format!("TMX 解析失败（{}）: {e}", args.path))
            })?;
            Ok(PyTilemap {
                map: RefCell::new(map),
            })
        }
    }

    /// 构造参（TMX 资源路径——sfx 密文链）。
    #[derive(FromArgs)]
    pub(crate) struct TilemapArgs {
        #[pyarg(any)]
        pub(crate) path: String,
    }
}
