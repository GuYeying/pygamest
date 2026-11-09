//! pygame.video 绑定（2026-10-24 自 native_pygame.rs 拆分；body 原样搬运）。
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
    pub mod video {
        use rustpython_derive::pyclass;
        use rustpython_vm::types::Constructor;
        use rustpython_vm::{
            PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
            builtins::{PyStrRef, PyType},
        };
        use starfish_pygame::video::{VideoSystem as SfVideoSystem, Video as SfVideo};
        use crate::rpy::pygame::pygame::PySurface;

        /// 路径规则单一事实源（io 同语义：相对路径 → 基准目录拼接，
        /// 绝对路径原样）——桌面/移动专属；Web 端 v1 站桩不走到此处
        #[cfg(not(target_arch = "wasm32"))]
        fn resolve_path(p: &str) -> String {
            starfish_io::resolve(p).to_string_lossy().into_owned()
        }

        /// pygame.video.VideoSystem——解码系统入口（set_mode 后创建）。
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame.video", name = "VideoSystem", unhashable = true)]
        pub(crate) struct PyVideoSystem {
            pub(crate) inner: SfVideoSystem,
        }

        impl std::fmt::Debug for PyVideoSystem {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("VideoSystem(<decoder>)")
            }
        }

        #[pyclass(with(Constructor))]
        impl PyVideoSystem {
            /// open(path) -> Video：打开媒体文件（缺失/不支持格式抛 ValueError）。
            #[pymethod]
            fn open(&self, path: PyStrRef, vm: &VirtualMachine) -> PyResult<PyRef<PyVideo>> {
                let p = path
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("路径含非 UTF-8 字符"))?;
                #[cfg(target_arch = "wasm32")]
                {
                    // Web 端 v1 站桩（WebCodecs 播放管线后续批次）——与
                    // dialog/io 同款显式 ValueError 语义，不落 unimplemented panic
                    let _ = p;
                    return Err(vm.new_value_error(
                        "pygame.video: Web 端 v1 不开放（WebCodecs 播放管线后续批次）",
                    ));
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let rp = resolve_path(p);
                    let v = self
                        .inner
                        .open(rp.clone())
                        .map_err(|e| {
                            vm.new_value_error(format!("video.open 失败: {e:?} path={rp}"))
                        })?;
                    Ok(PyVideo { inner: std::cell::RefCell::new(v) }.into_ref(&vm.ctx))
                }
            }
        }

        impl Constructor for PyVideoSystem {
            type Args = ();

            fn py_new(
                _cls: &Py<PyType>,
                _args: Self::Args,
                _vm: &VirtualMachine,
            ) -> PyResult<Self> {
                Ok(Self { inner: SfVideoSystem::new() })
            }
        }

        /// pygame.video.Video——已打开的媒体（update 驱动解码推进）。
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame.video", name = "Video", unhashable = true)]
        pub(crate) struct PyVideo {
            pub(crate) inner: std::cell::RefCell<SfVideo>,
        }

        impl std::fmt::Debug for PyVideo {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let (w, h) = self.inner.borrow().size();
                f.write_str(&format!("Video({}x{})", w, h))
            }
        }

        #[pyclass]
        impl PyVideo {
            /// update(dt_seconds)：推进解码（dt = 距上次调用的秒数；
            /// 解码错误抛 ValueError）。
            #[pymethod]
            fn update(&self, dt: f64, vm: &VirtualMachine) -> PyResult<()> {
                self.inner
                    .borrow_mut()
                    .update(std::time::Duration::from_secs_f64(dt.max(0.0)))
                    .map_err(|e| vm.new_value_error(format!("video.update 失败: {e:?}")))
            }

            /// size() -> (w, h)：视频原始尺寸。
            #[pymethod]
            fn size(&self) -> (u32, u32) {
                self.inner.borrow().size()
            }

            /// position() -> float：当前播放位置（秒）。
            #[pymethod]
            fn position(&self) -> f64 {
                self.inner.borrow().position().as_secs_f64()
            }

            /// ended() -> bool：是否播放到尾。
            #[pymethod]
            fn ended(&self) -> bool {
                self.inner.borrow().ended()
            }

            /// frame() -> Surface | None：当前帧 Surface 包装（零拷贝直通
            /// ——draw_surface 直采样；未出首帧 = None）。纹理同尺寸复用，
            /// 包装跨帧缓存（代数判据），零 bind 扰动
            #[pymethod]
            fn frame(&self, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
                let Some(s) = self.inner.borrow().frame() else {
                    return Ok(vm.ctx.none());
                };
                Ok(PySurface { surface: s }.into_ref(&vm.ctx).to_owned().into())
            }

            /// has_audio() -> bool：是否含音轨。
            #[pymethod]
            fn has_audio(&self) -> bool {
                self.inner.borrow().has_audio()
            }

            /// set_video_enabled(enabled)：视频轨开关（纯音轨模式）。
            #[pymethod]
            fn set_video_enabled(&self, enabled: bool) {
                self.inner.borrow_mut().set_video_enabled(enabled)
            }

            /// video_enabled() -> bool。
            #[pymethod]
            fn video_enabled(&self) -> bool {
                self.inner.borrow().video_enabled()
            }

            /// set_audio_volume(v)：音量 0.0..1.0。
            #[pymethod]
            fn set_audio_volume(&self, v: f32) {
                self.inner.borrow().set_audio_volume(v)
            }

            /// set_muted(m)：静音开关。
            #[pymethod]
            fn set_muted(&self, m: bool) {
                self.inner.borrow().set_muted(m)
            }
        }
    }

pub(crate) use video::module_def;
