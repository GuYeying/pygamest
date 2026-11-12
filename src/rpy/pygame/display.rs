//! pygame.display 绑定（2026-10-24 自 native_pygame.rs 拆分；body 原样搬运）。
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
    pub mod display {
        use crate::rpy::pygame::pygame::PyScreen;
        use crate::rpy::bridge;
        use crate::rpy::args::SfPoint;
        use rustpython_derive::FromArgs;
        use rustpython_vm::convert::TryFromObject;
        use rustpython_vm::{
            PyObjectRef, PyPayload, PyRef, PyResult, VirtualMachine,
            builtins::{PyStr, PyTupleRef, PyStrRef},
        };

        // ── 窗口旗标（locals::display 透传；set_mode 当前收下不解析，前向兼容）──
        #[pyattr]
        pub(crate) const RESIZABLE: u32 = starfish_pygame::display::RESIZABLE;
        #[pyattr]
        pub(crate) const FULLSCREEN: u32 = starfish_pygame::display::FULLSCREEN;
        #[pyattr]
        pub(crate) const NOFRAME: u32 = starfish_pygame::display::NOFRAME;
        #[pyattr]
        pub(crate) const HIDDEN: u32 = starfish_pygame::display::HIDDEN;
        #[pyattr]
        pub(crate) const ALWAYS_ON_TOP: u32 = starfish_pygame::display::ALWAYS_ON_TOP;
        #[pyattr]
        pub(crate) const TRANSPARENT: u32 = starfish_pygame::display::TRANSPARENT;
        #[pyattr]
        pub(crate) const VULKAN: u32 = starfish_pygame::display::VULKAN;
        #[pyattr]
        pub(crate) const DIRECTX: u32 = starfish_pygame::display::DIRECTX;
        #[pyattr]
        pub(crate) const METAL: u32 = starfish_pygame::display::METAL;
        #[pyattr]
        pub(crate) const OPENGLES: u32 = starfish_pygame::display::OPENGLES;

        /// set_mode(size, flags=0) → Screen 占位（真身由宿主壳先行装配，
        /// 此处记录需求；flags 批次 O 落地——位集/序列翻译到运行时窗口
        /// 旗标（apply_window_flags：RESIZABLE/FULLSCREEN/NOFRAME 即时
        /// 生效，创建期旗标垫底收下）。
        #[derive(FromArgs)]
        struct SetModeArgs {
            #[pyarg(any)]
            size: SfPoint,
            #[pyarg(any, optional)]
            flags: Option<PyObjectRef>,
        }

        /// flags 实参 → 位集（int 位掩码或 [位, ...] 序列，pygame 两形态）
        fn flags_bits(flags: &PyObjectRef, vm: &VirtualMachine) -> PyResult<u32> {
            if let Ok(bits) = u32::try_from_object(vm, flags.clone()) {
                return Ok(bits);
            }
            let parts: Vec<u32> = vm.extract_elements_with(flags, |o| {
                u32::try_from_object(vm, o.clone())
                    .map_err(|_| vm.new_type_error("flags 项应为 int 旗标位"))
            })?;
            Ok(parts.into_iter().fold(0u32, |acc, b| acc | b))
        }

        #[pyfunction]
        fn set_mode(args: SetModeArgs, vm: &VirtualMachine) -> PyResult<PyRef<PyScreen>> {
            // 启动诊断（单行打点：脚本 set_mode 的落地耗时）
            let t = starfish_std::debug::Stopwatch::now();
            // 完整窗口请求（批次 O + 尺寸扩展）：尺寸跟随（set_size）+
            // 旗标运行时生效（apply_window_request）
            let bits = match &args.flags {
                Some(f) => flags_bits(f, vm)?,
                None => 0,
            };
            bridge::devlog(&format!(
                "[pygame] display.set_mode {:?} 旗标 {bits:#b}（真身由宿主壳装配）",
                (args.size.0, args.size.1)
            ));
            starfish_pygame::display::apply_window_request(
                (args.size.0.max(0) as u32, args.size.1.max(0) as u32),
                bits,
            );
            bridge::devlog(&format!(
                "[set_mode] 落地 {}ms",
                t.ms()
            ));
            Ok(PyScreen.into_ref(&vm.ctx))
        }

        #[pyfunction]
        fn flip(_vm: &VirtualMachine) {
            starfish_pygame::display::flip();
        }

        /// set_opacity(v)：窗口整体不透明度（0.0 透明 .. 1.0 不透明）。
        /// 桌宠/悬浮件形态；与 TRANSPARENT 创建旗标正交（后者让 alpha 0
        /// 区域透出桌面，前者调窗口整体 alpha）。
        #[pyfunction]
        fn set_opacity(opacity: f32, _vm: &VirtualMachine) {
            starfish_pygame::display::set_opacity(opacity)
        }

        /// set_caption(title)：窗口标题。
        #[pyfunction]
        fn set_caption(title: PyStrRef, vm: &VirtualMachine) -> PyResult<()> {
            let t = title
                .to_str()
                .ok_or_else(|| vm.new_value_error("标题含非 UTF-8 字符"))?;
            starfish_pygame::display::set_caption(t);
            Ok(())
        }

        /// set_orientation(orientation)：请求屏幕方向（2026-11-12 竖屏
        /// 应用支持）。orientation = "portrait" / "landscape" /
        /// "sensor_portrait" / "sensor_landscape" / "sensor" /
        /// "full_sensor" / "unspecified"。Android 实装（Activity 级旋转
        /// ——运行时覆盖 manifest，宿主 configChanges 含 orientation 时
        /// 只重表面）；桌面/web = no-op；OHOS = 站桩 Err。
        #[pyfunction]
        fn set_orientation(orientation: PyStrRef, vm: &VirtualMachine) -> PyResult<()> {
            let o = orientation
                .to_str()
                .ok_or_else(|| vm.new_value_error("方向名含非 UTF-8 字符"))?;
            starfish_pygame::display::set_orientation(o)
                .map_err(|e| vm.new_runtime_error(e))
        }

        /// set_click_through(enable)：鼠标事件穿透开关（点击落到下层
        /// 窗口——桌宠形态配套；穿透设计稿 M1。⚠️ 穿透后窗口收不到
        /// 任何鼠标事件——恢复须脚本逻辑/外部通道，防自锁）。
        /// ⚠️ feature `click-through` 门控——未启用构建无此 API
        /// （ImportError = 编译期即知平台/构建不支持）。
        #[cfg(feature = "click-through")]
        #[pyfunction]
        fn set_click_through(enable: bool, vm: &VirtualMachine) -> PyResult<()> {
            starfish_pygame::display::set_click_through(enable)
                .map_err(|e| vm.new_runtime_error(e))
        }

        /// get_click_through() -> bool：窗口当前是否鼠标穿透（OS 位读回）。
        #[cfg(feature = "click-through")]
        #[pyfunction]
        fn get_click_through(_vm: &VirtualMachine) -> bool {
            starfish_pygame::display::get_click_through()
        }

        /// set_skip_taskbar(enable)：任务栏（Dock）隐藏开关（桌宠形态
        /// 配套——Win=WS_EX_TOOLWINDOW 位切换（Alt+Tab 一并隐藏）、
        /// macOS=Dock 级 setActivationPolicy 切换；Linux Wayland 站桩
        /// Err。⚠️ feature `skip-taskbar` 门控——未启用构建无此 API）。
        #[cfg(feature = "skip-taskbar")]
        #[pyfunction]
        fn set_skip_taskbar(enable: bool, vm: &VirtualMachine) -> PyResult<()> {
            starfish_pygame::display::set_skip_taskbar(enable)
                .map_err(|e| vm.new_runtime_error(e))
        }

        /// get_skip_taskbar() -> bool：窗口当前是否任务栏隐藏（OS 状态
        /// 读回；stub 平台恒 False）。
        #[cfg(feature = "skip-taskbar")]
        #[pyfunction]
        fn get_skip_taskbar(_vm: &VirtualMachine) -> bool {
            starfish_pygame::display::get_skip_taskbar()
        }

        /// set_tray(icon, tooltip="", menu=None) 收参（icon = 离屏
        /// Surface——RGBA 回读慢路径；menu = [(id, label), ...] 平铺——
        /// 收 PyObjectRef 列表手工拆元组，RP FromArgs 不支持元组列表）。
        #[derive(FromArgs)]
        struct TraySetArgs {
            /// 图标（离屏 Surface——Screen 背板不回读）
            icon: PyRef<crate::rpy::pygame::pygame::PySurface>,
            /// 悬停提示（缺省无）
            #[pyarg(any, optional)]
            tooltip: Option<PyStrRef>,
            /// 平铺菜单项 (id, label) 列表（缺省/空表 = 纯图标）
            #[pyarg(any, optional)]
            menu: Option<Vec<PyObjectRef>>,
        }

        /// set_tray(icon, tooltip="", menu=None)：安装（或替换）系统托盘
        /// 图标（桌宠形态配套——左键=trayclick 事件、右键=弹菜单、菜单
        /// 点击=traymenu 事件，经 pygame.event.get() 上浮。⚠️ feature
        /// `tray` 门控——未启用构建无此 API）。
        #[cfg(feature = "tray")]
        #[pyfunction]
        fn set_tray(args: TraySetArgs, vm: &VirtualMachine) -> PyResult<()> {
            let tooltip = match &args.tooltip {
                Some(t) => t
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("提示文本含非 UTF-8 字符"))?,
                None => "",
            };
            let mut menu: Vec<(String, String)> = Vec::new();
            for item in args.menu.unwrap_or_default() {
                let tuple = PyTupleRef::try_from_object(vm, item)
                    .map_err(|_| vm.new_type_error("menu 项须为 (id, label) 元组"))?;
                if tuple.len() != 2 {
                    return Err(vm.new_value_error(format!(
                        "menu 项须为二元组 (id, label)，收到 {} 元",
                        tuple.len()
                    )));
                }
                let id: PyStrRef = tuple
                    .get(0)
                    .cloned()
                    .ok_or_else(|| vm.new_value_error("menu 项索引越界"))?
                    .downcast::<PyStr>()
                    .map_err(|_| vm.new_type_error("菜单 id 须为 str"))?;
                let label: PyStrRef = tuple
                    .get(1)
                    .cloned()
                    .ok_or_else(|| vm.new_value_error("menu 项索引越界"))?
                    .downcast::<PyStr>()
                    .map_err(|_| vm.new_type_error("菜单文本须为 str"))?;
                let id = id
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("菜单 id 含非 UTF-8 字符"))?
                    .to_owned();
                let label = label
                    .to_str()
                    .ok_or_else(|| vm.new_value_error("菜单文本含非 UTF-8 字符"))?
                    .to_owned();
                menu.push((id, label));
            }
            starfish_pygame::display::set_tray(&args.icon.surface, tooltip, &menu)
                .map_err(|e| vm.new_runtime_error(e))
        }

        /// remove_tray()：移除托盘图标（未安装时空操作）。
        #[cfg(feature = "tray")]
        #[pyfunction]
        fn remove_tray(vm: &VirtualMachine) -> PyResult<()> {
            starfish_pygame::display::remove_tray().map_err(|e| vm.new_runtime_error(e))
        }

        /// get_init() -> bool：set_mode 是否已调用（display 初始化态）。
        #[pyfunction]
        fn get_init(_vm: &VirtualMachine) -> bool {
            starfish_pygame::display::get_init()
        }

        /// quit()：关闭显示（清窗口/屏句柄；pygame 标准对——可与 init 态
        /// 配合做"重开窗口"生命周期）。
        #[pyfunction]
        fn quit(_vm: &VirtualMachine) {
            starfish_pygame::display::quit();
        }

        /// map_point(x, y) -> (lx, ly)：物理点 → 逻辑点（窗口/屏的呈现
        /// 映射逆变换——等比缩放 + 居中）。脚本自处理原始触点/外部坐标
        /// 时用（泵层事件坐标已自动映射，常规事件流无需此口）。
        #[pyfunction]
        fn map_point(x: f64, y: f64, _vm: &VirtualMachine) -> (f64, f64) {
            starfish_pygame::display::map_point(x, y)
        }
    }

pub(crate) use display::module_def;
