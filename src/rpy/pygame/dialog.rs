//! pygame.dialog 绑定（2026-10-24 自 native_pygame.rs 拆分；body 原样搬运）。
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
    pub mod dialog {
        use rustpython_derive::pyclass;
        use rustpython_vm::convert::TryFromObject;
        use rustpython_vm::{PyObjectRef, PyPayload, PyResult, VirtualMachine, builtins::PyStrRef};

        use starfish_dialog::PickedFile;

        /// PickedFile：用户选取的文件（桌面持路径惰性读盘）。
        #[derive(PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame.dialog", name = "PickedFile", unhashable = true)]
        pub(crate) struct PyPickedFile(PickedFile);

        impl std::fmt::Debug for PyPickedFile {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("PickedFile")
            }
        }

        #[pyclass]
        impl PyPickedFile {
            /// 文件名（含扩展名）。
            #[pymethod]
            fn name(&self) -> String {
                self.0.name().to_owned()
            }

            /// read() -> bytes：读取全部字节。
            #[pymethod]
            fn read(&self, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
                let data = self
                    .0
                    .read()
                    .map_err(|e| vm.new_value_error(format!("dialog 读取失败: {e}")))?;
                Ok(vm.ctx.new_bytes(data).into())
            }
        }

        /// 过滤项 [(展示名, [扩展名, ...]), ...] → dialog Filter 标记
        /// 生命周期（&'static）——一次性用户动作，Box::leak 有意泄漏
        ///（次数有界，进程级回收）。
        fn leak_filters(
            filters: &[(String, Vec<String>)],
        ) -> Vec<(&'static str, &'static [&'static str])> {
            filters
                .iter()
                .map(|(name, exts)| {
                    let name: &'static str = Box::leak(name.clone().into_boxed_str());
                    let exts: Vec<&'static str> = exts
                        .iter()
                        .map(|e| Box::leak(e.clone().into_boxed_str()) as &'static str)
                        .collect();
                    (name, Box::leak(exts.into_boxed_slice()) as &'static [&'static str])
                })
                .collect()
        }

        /// open_file(title=None, filters=None) -> PickedFile | None
        ///（None = 用户取消。filters = [("文本文件", ["txt", "md"]), ...]）
        #[pyfunction]
        fn open_file(args: OpenFileArgs, vm: &VirtualMachine) -> PyResult<Option<PyPickedFile>> {
            #[cfg(not(any(
                target_os = "windows",
                target_os = "linux",
                target_os = "macos"
            )))]
            return Err(vm.new_value_error(
                "pygame.dialog: v1 同步对话框为桌面专属（Web/Android 走异步 job API，后续批次）",
            ));
            #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
            {
                // 过滤项手工解包（FromArgs 通道对嵌套 Vec<tuple> 不通，
                // RP 亦无 Vec/tuple 的 TryFromObject——downcast 元组后
                // as_slice 逐元素提取，extract_elements_with 同 sndarray）
                let filters: Vec<(String, Vec<String>)> = match &args.filters {
                    Some(o) => vm.extract_elements_with(o, |el| {
                        let t = el.downcast::<rustpython_vm::builtins::PyTuple>()
                            .map_err(|_| vm.new_type_error("过滤项应为 (展示名, [扩展名...]) 二元组"))?;
                        let items = t.as_slice();
                        if items.len() != 2 {
                            return Err(vm.new_type_error("过滤项应为二元组 (展示名, [扩展名...])"));
                        }
                        let name = items[0]
                            .downcast_ref::<rustpython_vm::builtins::PyStr>()
                            .ok_or_else(|| vm.new_type_error("过滤项[0] 应为展示名 str"))?
                            .to_str()
                            .ok_or_else(|| vm.new_value_error("展示名含非 UTF-8 字符"))?
                            .to_owned();
                        let exts: Vec<String> = vm.extract_elements_with(&items[1], |e| {
                            e.downcast_ref::<rustpython_vm::builtins::PyStr>()
                                .ok_or_else(|| vm.new_type_error("扩展名应为 str"))?
                                .to_str()
                                .ok_or_else(|| vm.new_value_error("扩展名含非 UTF-8 字符"))
                                .map(String::from)
                        })?;
                        Ok((name, exts))
                    })?,
                    None => Vec::new(),
                };
                let filters = leak_filters(&filters);
                let picked = pollster::block_on(starfish_dialog::pick_file(
                    args.title.as_deref(),
                    &filters,
                ))
                .map_err(|e| vm.new_value_error(format!("dialog: {e}")))?;
                Ok(picked.map(PyPickedFile))
            }
        }

        #[derive(rustpython_derive::FromArgs)]
        pub(crate) struct OpenFileArgs {
            /// 对话框标题（None = 系统默认）
            #[pyarg(any, optional)]
            pub(crate) title: Option<String>,
            /// 过滤项 [(展示名, [扩展名...]), ...]（PyObjectRef 手工解包）
            #[pyarg(any, optional)]
            pub(crate) filters: Option<PyObjectRef>,
        }

        /// save_bytes(file_name, data) -> bool：保存到用户指定位置
        ///（True = 已保存 / False = 用户取消。桌面 = 另存为对话框；
        /// Web 语义 = 触发下载——v1 桌面专属）。
        #[pyfunction]
        fn save_bytes(
            file_name: PyStrRef,
            data: PyObjectRef,
            vm: &VirtualMachine,
        ) -> PyResult<bool> {
            #[cfg(not(any(
                target_os = "windows",
                target_os = "linux",
                target_os = "macos"
            )))]
            return Err(vm.new_value_error(
                "pygame.dialog: v1 同步对话框为桌面专属（Web/Android 走异步 job API，后续批次）",
            ));
            #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
            {
                let name = file_name
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("文件名含非 UTF-8 字符"))?;
                let bytes = data_bytes(data, vm)?;
                let saved = pollster::block_on(starfish_dialog::save_bytes(name, bytes))
                    .map_err(|e| vm.new_value_error(format!("dialog: {e}")))?;
                Ok(saved.is_some())
            }
        }

        // ── 异步 job 三原生口（2026-11-10 异步面定约：open_file/save_bytes
        // 的 await 形态机制层——后台线程跑模态对话框，帧轮询就绪，取走时
        // 构造 PyObject。Python 面 = testhost.rs FRAME_BRIDGE 的
        // _DialogOpen/_DialogSave shim 挂载）──

        /// 后台任务注册表（结果载荷：打开 = Option<PickedFile>（None =
        /// 用户取消）；保存 = bool）
        static OPEN_JOBS: std::sync::OnceLock<crate::rpy::jobs::Registry<Option<PickedFile>>> =
            std::sync::OnceLock::new();
        static SAVE_JOBS: std::sync::OnceLock<crate::rpy::jobs::Registry<bool>> =
            std::sync::OnceLock::new();

        // ── Android 形态：SAF 轮询槽（阻塞式在 SAF 覆盖期会停摆事件泵
        // ——starfish-dialog 实测定案；start/poll 全在引擎线程，覆盖期由
        // 宿主按 starfish_dialog::jobs_active 豁免 surface 销毁停引擎）──
        #[cfg(target_os = "android")]
        mod saf_slots {
            use super::*;
            use std::sync::Mutex;

            /// 打开槽：PickJob 在位待轮询 / 或已出终值待取
            struct OpenSlot {
                job: Mutex<Option<starfish_dialog::PickJob>>,
                result: Mutex<Option<Result<Option<PickedFile>, String>>>,
            }
            struct SaveSlot {
                job: Mutex<Option<starfish_dialog::SaveJob>>,
                result: Mutex<Option<Result<bool, String>>>,
            }

            static OPEN: std::sync::Mutex<Vec<std::sync::Arc<OpenSlot>>> =
                std::sync::Mutex::new(Vec::new());
            static SAVE: std::sync::Mutex<Vec<std::sync::Arc<SaveSlot>>> =
                std::sync::Mutex::new(Vec::new());

            pub(super) fn open_start(
                title: Option<String>,
                filters: Vec<(String, Vec<String>)>,
            ) -> Result<i64, String> {
                let filters = leak_filters(&filters);
                let job = starfish_dialog::pick_file_start(title.as_deref(), &filters)
                    .map_err(|e| format!("dialog: {e}"))?;
                let slot = std::sync::Arc::new(OpenSlot {
                    job: Mutex::new(Some(job)),
                    result: Mutex::new(None),
                });
                let mut v = OPEN.lock().unwrap();
                v.push(slot);
                Ok((v.len() - 1) as i64)
            }

            pub(super) fn save_start(name: String, data: Vec<u8>) -> Result<i64, String> {
                let job = starfish_dialog::save_bytes_start(&name, data)
                    .map_err(|e| format!("dialog: {e}"))?;
                let slot = std::sync::Arc::new(SaveSlot {
                    job: Mutex::new(Some(job)),
                    result: Mutex::new(None),
                });
                let mut v = SAVE.lock().unwrap();
                v.push(slot);
                Ok((v.len() - 1) as i64)
            }

            /// 轮询推进（try_result 出终值即转存结果槽）
            pub(super) fn poll(h: i64) -> bool {
                let open_done = {
                    let v = OPEN.lock().unwrap();
                    match v.get(h as usize) {
                        Some(s) => {
                            let mut jg = s.job.lock().unwrap();
                            if let Some(j) = jg.as_mut() {
                                if let Some(r) = j.try_result() {
                                    let mapped = r.map_err(|e| format!("dialog: {e}"));
                                    *s.result.lock().unwrap() = Some(mapped);
                                    *jg = None;
                                }
                            }
                            s.result.lock().unwrap().is_some()
                        }
                        None => true, // 陈旧句柄
                    }
                };
                if open_done {
                    return true;
                }
                let v = SAVE.lock().unwrap();
                match v.get(h as usize) {
                    Some(s) => {
                        let mut jg = s.job.lock().unwrap();
                        if let Some(j) = jg.as_mut() {
                            if let Some(r) = j.try_result() {
                                let mapped = r
                                    .map(|saved| saved.is_some())
                                    .map_err(|e| format!("dialog: {e}"));
                                *s.result.lock().unwrap() = Some(mapped);
                                *jg = None;
                            }
                        }
                        s.result.lock().unwrap().is_some()
                    }
                    None => true,
                }
            }

            pub(super) fn open_take(h: i64) -> Result<Option<PickedFile>, String> {
                let v = OPEN.lock().unwrap();
                let s = v.get(h as usize).ok_or("SAF 打开槽句柄不存在")?;
                s.result
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap_or_else(|| Err("SAF 打开槽未完成".into()))
            }

            pub(super) fn save_take(h: i64) -> Result<bool, String> {
                let v = SAVE.lock().unwrap();
                let s = v.get(h as usize).ok_or("SAF 保存槽句柄不存在")?;
                s.result
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap_or_else(|| Err("SAF 保存槽未完成".into()))
            }
        }

        /// 过滤项 PyObjectRef 手工解包（FromArgs 通道对嵌套 Vec<tuple> 不通）
        fn unpack_filters(
            filters: &Option<PyObjectRef>,
            vm: &VirtualMachine,
        ) -> PyResult<Vec<(String, Vec<String>)>> {
            match filters {
                Some(o) => vm.extract_elements_with(o, |el| {
                    let t = el
                        .downcast::<rustpython_vm::builtins::PyTuple>()
                        .map_err(|_| {
                            vm.new_type_error("过滤项应为 (展示名, [扩展名...]) 二元组")
                        })?;
                    let items = t.as_slice();
                    if items.len() != 2 {
                        return Err(vm.new_type_error("过滤项应为二元组 (展示名, [扩展名...])"));
                    }
                    let name = items[0]
                        .downcast_ref::<rustpython_vm::builtins::PyStr>()
                        .ok_or_else(|| vm.new_type_error("过滤项[0] 应为展示名 str"))?
                        .to_str()
                        .ok_or_else(|| vm.new_value_error("展示名含非 UTF-8 字符"))?
                        .to_owned();
                    let exts: Vec<String> = vm.extract_elements_with(&items[1], |e| {
                        e.downcast_ref::<rustpython_vm::builtins::PyStr>()
                            .ok_or_else(|| vm.new_type_error("扩展名应为 str"))?
                            .to_str()
                            .ok_or_else(|| vm.new_value_error("扩展名含非 UTF-8 字符"))
                            .map(String::from)
                    })?;
                    Ok((name, exts)
                    )
                }),
                None => Ok(Vec::new()),
            }
        }

        /// _open_start(title, filters) -> 句柄：桌面 = 后台线程模态框；
        /// Android = SAF 轮询槽（阻塞式在 SAF 覆盖期停摆事件泵——实测定案）
        #[pyfunction]
        fn _open_start(
            args: OpenFileArgs,
            vm: &VirtualMachine,
        ) -> PyResult<rustpython_vm::builtins::PyIntRef> {
            #[cfg(not(any(
                target_os = "windows",
                target_os = "linux",
                target_os = "macos",
                target_os = "android"
            )))]
            {
                let _ = (args, vm);
                return Err(vm.new_value_error(
                    "pygame.dialog: 对话框后端未接入（iOS/Web 后续批次）",
                ));
            }
            #[cfg(target_os = "android")]
            {
                let filters = unpack_filters(&args.filters, vm)?;
                let h = saf_slots::open_start(args.title.clone(), filters)
                    .map_err(|e| vm.new_value_error(e))?;
                return Ok(rustpython_vm::builtins::PyInt::from(h).into_ref(&vm.ctx));
            }
            #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
            {
                let filters = unpack_filters(&args.filters, vm)?;
                let filters = leak_filters(&filters);
                let title = args.title.clone();
                let reg = OPEN_JOBS.get_or_init(Default::default);
                let h = reg.spawn(move || {
                    pollster::block_on(starfish_dialog::pick_file(title.as_deref(), &filters))
                        .map_err(|e| format!("dialog: {e}"))
                });
                Ok(rustpython_vm::builtins::PyInt::from(h).into_ref(&vm.ctx))
            }
        }

        /// _save_start(file_name, data) -> 句柄：桌面 = 后台线程另存为；
        /// Android = SAF 轮询槽
        #[pyfunction]
        fn _save_start(
            file_name: PyStrRef,
            data: PyObjectRef,
            vm: &VirtualMachine,
        ) -> PyResult<rustpython_vm::builtins::PyIntRef> {
            #[cfg(not(any(
                target_os = "windows",
                target_os = "linux",
                target_os = "macos",
                target_os = "android"
            )))]
            {
                let _ = (file_name, data, vm);
                return Err(vm.new_value_error(
                    "pygame.dialog: 对话框后端未接入（iOS/Web 后续批次）",
                ));
            }
            let name = file_name
                .to_str()
                .ok_or_else(|| vm.new_value_error("文件名含非 UTF-8 字符"))?
                .to_owned();
            let bytes = data_bytes(data, vm)?;
            #[cfg(target_os = "android")]
            {
                let h = saf_slots::save_start(name, bytes)
                    .map_err(|e| vm.new_value_error(e))?;
                return Ok(rustpython_vm::builtins::PyInt::from(h).into_ref(&vm.ctx));
            }
            #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
            {
                let reg = SAVE_JOBS.get_or_init(Default::default);
                let h = reg.spawn(move || {
                    pollster::block_on(starfish_dialog::save_bytes(name.as_str(), bytes))
                        .map(|saved| saved.is_some())
                        .map_err(|e| format!("dialog: {e}"))
                });
                Ok(rustpython_vm::builtins::PyInt::from(h).into_ref(&vm.ctx))
            }
        }

        /// _poll(句柄) -> bool：job 就绪查询（帧轮询——桌面查线程注册
        /// 表；Android 顺带推进 SAF try_result 转存）
        #[pyfunction]
        fn _poll(handle: i64) -> bool {
            #[cfg(target_os = "android")]
            {
                saf_slots::poll(handle)
            }
            #[cfg(not(target_os = "android"))]
            {
                OPEN_JOBS
                    .get()
                    .map(|r| r.poll(handle))
                    .unwrap_or(false)
                    || SAVE_JOBS.get().map(|r| r.poll(handle)).unwrap_or(false)
            }
        }

        /// _open_take(句柄) -> PickedFile | None：取走结果（取消 = None）
        #[pyfunction]
        fn _open_take(
            handle: i64,
            vm: &VirtualMachine,
        ) -> PyResult<Option<PyPickedFile>> {
            #[cfg(target_os = "android")]
            {
                let picked = saf_slots::open_take(handle).map_err(|e| vm.new_value_error(e))?;
                return Ok(picked.map(PyPickedFile));
            }
            #[cfg(not(target_os = "android"))]
            {
                let reg = OPEN_JOBS.get().ok_or_else(|| {
                    vm.new_value_error("pygame.dialog: 打开任务表未初始化")
                })?;
                let picked = reg.take(handle).map_err(|e| vm.new_value_error(e))?;
                Ok(picked.map(PyPickedFile))
            }
        }

        /// _save_take(句柄) -> bool：取走保存结果
        #[pyfunction]
        fn _save_take(handle: i64, vm: &VirtualMachine) -> PyResult<bool> {
            #[cfg(target_os = "android")]
            {
                return saf_slots::save_take(handle).map_err(|e| vm.new_value_error(e));
            }
            #[cfg(not(target_os = "android"))]
            {
                let reg = SAVE_JOBS
                    .get()
                    .ok_or_else(|| vm.new_value_error("pygame.dialog: 保存任务表未初始化"))?;
                reg.take(handle).map_err(|e| vm.new_value_error(e))
            }
        }

        fn data_bytes(data: PyObjectRef, vm: &VirtualMachine) -> PyResult<Vec<u8>> {
            if let Some(s) = data.downcast_ref::<rustpython_vm::builtins::PyStr>() {
                return Ok(s
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("内容含非 UTF-8 字符"))?
                    .as_bytes()
                    .to_vec());
            }
            data.try_bytes_like(vm, |b| b.to_vec())
        }
    }

    // ── gfxdraw：SDL_gfx 风格兼容签名面（契约原则 3——一个实现多个
    // 兼容名；gfxdraw 独立栅格与 AA 语义已并入 draw/wgpu 采样，本模块
    // 只是签名适配：目标会话在前、坐标分离、颜色在后；aa* = 兼容别名，

pub(crate) use dialog::module_def;
