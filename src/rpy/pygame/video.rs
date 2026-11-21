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
        use starfish_video::VideoModule as SfVideoModule;
        use crate::rpy::pygame::pygame::PySurface;

        // ── wasm 异步 open 交互盒（Video 非 Send 也无需 Send——wasm 单
        // 线程，spawn_local 即主线程；桌面侧 = OPEN_JOBS 线程桥，两侧
        // 语义同构：_open_start 发起 / _poll 轮询 / _open_take 取走）──
        #[cfg(target_arch = "wasm32")]
        thread_local! {
            static WASM_JOBS: std::cell::RefCell<Vec<Option<starfish_video::Video>>> =
                const { std::cell::RefCell::new(Vec::new()) };
        }

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

        // ── 异步 job 三原生口（2026-11-10 异步面定约：pygame.video.open
        // 的 await 形态机制层——后台线程跑 open（MediaCodec configure 可达
        // 百毫秒级），帧轮询就绪取走。Python 面 = testhost.rs FRAME_BRIDGE
        // 的 _VideoOpen shim 挂载；VideoSystem.open 同步口保留）──

        /// 跨线程交接盒（SfVideo 非 Send：解码器 trait 对象 + 帧包装
        /// Surface 的 RefCell。交接健全性：worker 线程在 Mutex 临界区内
        /// 写入 + Release 序发布 done 位；引擎线程 Acquire 读到 done 后
        /// 才经同一 Mutex 取走——happens-before 成立，单生产单消费，
        /// 之后仅引擎线程触碰）
        struct SendVideo(SfVideo);
        // SAFETY: 见上注——Mutex + Release/Acquire 交接序，消费后独占
        unsafe impl Send for SendVideo {}

        /// 后台任务注册表（载荷：打开成功 = SendVideo 交接盒）
        static OPEN_JOBS: std::sync::OnceLock<crate::rpy::jobs::Registry<SendVideo>> =
            std::sync::OnceLock::new();

        /// _open_start(path) -> 句柄：后台线程跑 open（含 io 路径解析）。
        /// wasm = **WebCodecs 异步接线**（2026-11-21 批次）：spawn_local
        /// 驱动 VideoModule::open（fetch+demux+WebCodecs 状态机），完成后
        /// Video 落 thread_local 交互盒——_poll 轮询就绪 / _open_take 取走
        #[pyfunction]
        fn _open_start(path: PyStrRef, vm: &VirtualMachine) -> PyResult<rustpython_vm::builtins::PyIntRef> {
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen_futures::spawn_local;
                let _ = vm;
                let p = path.to_str().unwrap_or_default().to_string();
                // VideoSystem::new 构造期取 GPU 槽（wasm 主线程 = 同步段 ✓）
                let sys = SfVideoSystem::new();
                let vm_mod = std::rc::Rc::new(sys.module());
                let handle = WASM_JOBS.with(|jobs| {
                    let h = jobs.borrow().len() + 1;   // 句柄 = 索引 + 1（0 保留）
                    jobs.borrow_mut().push(None);      // Pending 占位
                    h
                });
                spawn_local(async move {
                    let opened = vm_mod.open(p).await.ok();
                    WASM_JOBS.with(|jobs| {
                        jobs.borrow_mut()[handle as usize - 1] = opened;
                    });
                });
                return Ok(rustpython_vm::builtins::PyInt::from(handle as i64).into_ref(&vm.ctx));
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let p = path
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("路径含非 UTF-8 字符"))?
                    .to_owned();
                // System 构造在引擎线程（VideoSystem::new 取线程局部 GPU 槽
                // ——worker 线程无槽必 panic）；open 本体只吃 device/queue
                // 的 Arc 克隆（wgpu 句柄 Send+Sync），移进 worker 安全
                let sys = SfVideoSystem::new();
                let reg = OPEN_JOBS.get_or_init(Default::default);
                let h = reg.spawn(move || {
                    let rp = resolve_path(&p);
                    sys.open(rp)
                        .map(SendVideo)
                        .map_err(|e| format!("video.open 失败: {e:?} path={p}"))
                });
                Ok(rustpython_vm::builtins::PyInt::from(h).into_ref(&vm.ctx))
            }
        }

        /// _poll(句柄) -> bool：job 就绪查询（帧轮询）
        #[pyfunction]
        fn _poll(handle: i64) -> bool {
            #[cfg(target_arch = "wasm32")]
            {
                // wasm 交互盒：句柄位 Some = VideoModule::open 完成
                WASM_JOBS.with(|jobs| {
                    jobs.borrow()
                        .get(handle as usize - 1)
                        .map(|j| j.is_some())
                        .unwrap_or(false)
                })
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                OPEN_JOBS.get().map(|r| r.poll(handle)).unwrap_or(false)
            }
        }

        /// _open_take(句柄) -> Video：取走结果
        #[pyfunction]
        fn _open_take(handle: i64, vm: &VirtualMachine) -> PyResult<PyRef<PyVideo>> {
            #[cfg(target_arch = "wasm32")]
            {
                let taken = WASM_JOBS.with(|jobs| {
                    let idx = handle as usize - 1;
                    let mut jobs = jobs.borrow_mut();
                    jobs.get_mut(idx)?.take()
                });
                // BaseVideo → starfish-pygame 包装（from_base——wasm 完成侧
                // 接线口，2026-11-21 video wasm 接线批次）
                let v = taken
                    .map(starfish_pygame::video::Video::from_base)
                    .ok_or_else(|| {
                        vm.new_value_error("pygame.video: 打开任务未完成（_poll 后再取）")
                    })?;
                Ok(PyVideo {
                    inner: std::cell::RefCell::new(v),
                }
                .into_ref(&vm.ctx))
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let reg = OPEN_JOBS
                    .get()
                    .ok_or_else(|| vm.new_value_error("pygame.video: 打开任务表未初始化"))?;
                let v = reg.take(handle).map_err(|e| vm.new_value_error(e))?;
                Ok(PyVideo {
                    inner: std::cell::RefCell::new(v.0),
                }
                .into_ref(&vm.ctx))
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
