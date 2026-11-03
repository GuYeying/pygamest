//! pygame.image 绑定（2026-10-24 自 native_pygame.rs 拆分；body 原样搬运）。
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
    pub mod image {
        use rustpython_derive::pymodule;
        use rustpython_vm::{PyPayload, PyRef, PyResult, VirtualMachine};
        use starfish_pygame::image as sf_image;

        use crate::rpy::pygame::pygame::PySurface;

        #[pyfunction]
        fn get_extended() -> bool {
            sf_image::get_extended()
        }

        /// 路径加载（**全平台同语义**，2026-11-02 定约——Web 由壳启动期
        /// 预载表承载字节，starfish_io::read_sync 命中即解密返回）：缺
        /// 资源不崩溃定约 = 占位图顶上（品红棋盘格）；严格模式（
        /// STARFISH_STRICT_ASSETS）恢复真异常供 CI 测漏
        #[pyfunction]
        fn load(path: String, vm: &VirtualMachine) -> PyResult<PyRef<PySurface>> {
            match sf_image::load(&path) {
                Ok(s) => Ok(PySurface { surface: std::sync::Arc::new(s) }.into_ref(&vm.ctx)),
                Err(e) => {
                    // "缺资源不崩溃"定约：占位图顶上（品红棋盘格）；严格
                    // 模式（STARFISH_STRICT_ASSETS）恢复真异常供 CI 测漏
                    starfish_debug::log_error!(format!(
                        "[assets] 图片加载失败 {path}: {e:?}——占位图顶上"
                    ));
                    if sf_image::strict_assets() {
                        return Err(vm.new_value_error(format!("图片加载失败: {e:?}")));
                    }
                    Ok(PySurface {
                        surface: std::sync::Arc::new(sf_image::placeholder_surface(64)),
                    }
                    .into_ref(&vm.ctx))
                }
            }
        }

        /// PNG/JPEG/BMP/GIF/WebP 按内容解码（Web 首选入口）
        #[pyfunction]
        fn load_from_bytes(bytes: Vec<u8>, vm: &VirtualMachine) -> PyResult<PyRef<PySurface>> {
            match sf_image::load_from_bytes(&bytes) {
                Ok(s) => Ok(PySurface { surface: std::sync::Arc::new(s) }.into_ref(&vm.ctx)),
                Err(e) => {
                    starfish_debug::log_error!(format!(
                        "[assets] 图片解码失败（{} 字节）: {e:?}——占位图顶上",
                        bytes.len()
                    ));
                    if sf_image::strict_assets() {
                        return Err(vm.new_value_error(format!("图片解码失败: {e:?}")));
                    }
                    Ok(PySurface {
                        surface: std::sync::Arc::new(sf_image::placeholder_surface(64)),
                    }
                    .into_ref(&vm.ctx))
                }
            }
        }

        /// 桌面专用（block_on 回读落盘；格式按扩展名，无扩展名默认 PNG）
        #[cfg(not(target_arch = "wasm32"))]
        #[pyfunction]
        fn save(surf: PyRef<PySurface>, path: String, vm: &VirtualMachine) -> PyResult<()> {
            sf_image::save(&surf.surface, &path)
                .map_err(|e| vm.new_value_error(format!("图片保存失败: {e:?}")))
        }

        /// Web：save 须 save_async 异步语义（批次出栈）
        #[cfg(target_arch = "wasm32")]
        #[pyfunction]
        fn save(_surf: PyRef<PySurface>, _path: String, vm: &VirtualMachine) -> PyResult<()> {
            Err(vm.new_not_implemented_error("image.save 桌面专用（Web 须 save_async 异步语义，后续批次）"))
        }
    }

    // ── key：按下视图查询（K_* 常量在 pygame 顶层，见 KEY_TABLE）─────

pub(crate) use image::module_def;
