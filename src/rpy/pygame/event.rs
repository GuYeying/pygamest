//! pygame.event 绑定（2026-10-24 自 native_pygame.rs 拆分；body 原样搬运）。
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
    pub mod event {
        use rustpython_vm::builtins::PyModule;
        use starfish_we::KeyCode;
        use starfish_pygame::Event as EngineEvent;
        use rustpython_derive::{pyclass, pymodule};
        use rustpython_vm::types::{Comparable, PyComparisonOp, Representable};
        use rustpython_vm::function::PyComparisonValue;
        use rustpython_vm::{
            Py, PyObject, PyObjectRef, PyPayload, PyRef, PyResult, VirtualMachine,
            builtins::{PyStr, PyStrRef},
        };
        use crate::rpy::pygame::key::key::PyKey;
use crate::rpy::{bridge, args};


        // ── 常量（tag/键值定约与 bridge.rs 事件映射一致）──
        #[pyattr]
        pub(crate) const QUIT: &str = "quit";
        #[pyattr]
        pub(crate) const KEYDOWN: &str = "keydown";
        #[pyattr]
        pub(crate) const KEYUP: &str = "keyup";
        #[pyattr]
        pub(crate) const MOUSEMOTION: &str = "mousemotion";
        #[pyattr]
        pub(crate) const MOUSEBUTTONDOWN: &str = "mousebuttondown";
        #[pyattr]
        pub(crate) const MOUSEBUTTONUP: &str = "mousebuttonup";
        /// 滚轮（e.x = 横向 / e.y = 纵向刻度）
        #[pyattr]
        pub(crate) const MOUSEWHEEL: &str = "mousewheel";
        /// 窗口尺寸变化（e.w / e.h = 新逻辑尺寸）
        #[pyattr]
        pub(crate) const RESIZED: &str = "resized";
        /// 窗口焦点获得 / 失去
        #[pyattr]
        pub(crate) const FOCUSGAINED: &str = "focusgained";
        #[pyattr]
        pub(crate) const FOCUSLOST: &str = "focuslost";
        #[pyattr]
        pub(crate) const TEXTINPUT: &str = "textinput";
        // ── 托盘事件词汇（feature "tray" 门控——未启用构建无此常量；
        // 事件对象也只在托盘交互时产生）──
        /// 托盘菜单项点击（e.id = 菜单项 id）
        #[cfg(feature = "tray")]
        #[pyattr]
        pub(crate) const TRAYMENU: &str = "traymenu";
        /// 托盘图标单击（e.button = 1/2/3，e.x/e.y = 屏幕物理坐标）
        #[cfg(feature = "tray")]
        #[pyattr]
        pub(crate) const TRAYCLICK: &str = "trayclick";
        /// 托盘图标双击（Windows 专属形态）
        #[cfg(feature = "tray")]
        #[pyattr]
        pub(crate) const TRAYDOUBLECLICK: &str = "traydoubleclick";






        // ── Event：事件对象（pygame.event.get() 产物）。**全保真载荷**
        //    （2026-10-24 重构）：引擎 typed 枚举原样持有（egui 等事件
        //    逐条消费者可重建完整原始输入），预展开展示字段供 getset 免
        //    vm 取用。`type` 为 tag 字符串；`mod` = KMOD 位（真 pygame
        //    `e.mod` parity）；textinput 走 `text` 槽（旧形态塞 key 槽的
        //    hack 退役）；resized 走 w/h（旧形态塌缩成 "other"）──
        #[derive(Debug, PyPayload)]
        #[pyattr]
        #[pyclass(module = "pygame", name = "Event", unhashable = true)]
        pub(crate) struct PyEvent {
            pub(crate) ev: EngineEvent,
            pub(crate) typ: PyStrRef,
            pub(crate) key: PyObjectRef,
            pub(crate) x: i32,
            pub(crate) y: i32,
            pub(crate) button: i32,
            /// KMOD 位集（keydown/keyup；真 pygame `e.mod` parity）
            pub(crate) modifiers: u32,
            /// 自动重复（keydown）
            pub(crate) repeat: bool,
            /// 文本（textinput）
            pub(crate) text: PyStrRef,
            /// 托盘事件 id（traymenu = 菜单项 id；trayclick/traydouble
            /// = 托盘图标 id；其余 ""）
            pub(crate) id: PyStrRef,
            /// 尺寸（resized）
            pub(crate) w: i32,
            pub(crate) h: i32,
        }

        #[pyclass(with(Representable))]
        impl PyEvent {
            #[pygetset(name = "type")]
            fn typ(&self) -> PyStrRef {
                self.typ.clone()
            }

            #[pygetset]
            fn key(&self) -> PyObjectRef {
                self.key.clone()
            }

            #[pygetset]
            fn x(&self) -> i32 {
                self.x
            }

            #[pygetset]
            fn y(&self) -> i32 {
                self.y
            }

            #[pygetset]
            fn button(&self) -> i32 {
                self.button
            }

            /// 修饰键位集（KMOD_*；keydown/keyup 携带，其余 0）
            #[pygetset(name = "mod")]
            fn modifiers(&self) -> u32 {
                self.modifiers
            }

            /// 自动重复标记（keydown；其余 False）
            #[pygetset]
            fn repeat(&self) -> bool {
                self.repeat
            }

            /// 文本（textinput；其余 ""）
            #[pygetset]
            fn text(&self) -> PyStrRef {
                self.text.clone()
            }

            /// 托盘事件 id（traymenu/trayclick/traydoubleclick 携带；
            /// 其余 ""）
            #[pygetset]
            fn id(&self) -> PyStrRef {
                self.id.clone()
            }

            /// 宽（resized；其余 0）
            #[pygetset]
            fn w(&self) -> i32 {
                self.w
            }

            /// 高（resized；其余 0）
            #[pygetset]
            fn h(&self) -> i32 {
                self.h
            }
        }

        impl Representable for PyEvent {
            fn repr(zelf: &Py<Self>, vm: &VirtualMachine) -> PyResult<PyRef<PyStr>> {
                Ok(vm.ctx.new_str(format!(
                    "<Event {} key={} pos=({}, {}) button={}>",
                    zelf.typ,
                    zelf.key.repr(vm)?,
                    zelf.x,
                    zelf.y,
                    zelf.button
                )))
            }
        }

        impl PyEvent {
            /// 引擎事件 → PyEvent（**全保真**：枚举原样进载荷 + 展示字段
            /// 预展开。typed 键值定约批次 C：`e.key == pygame.key.K_x` 用
            /// Key 实例值相等比较（K_* 全表在 key 模块——`pygame.event.K_*`
            /// 不存在）；非键事件的 key 位为 int 0——textinput 的
            /// 文本走 `text` 槽）。
            pub(crate) fn from_engine(vm: &VirtualMachine, ev: EngineEvent) -> Self {
                let tag = |s: &str| -> PyStrRef { vm.ctx.intern_str(s).to_owned() };
                let int_obj = |v: u32| -> PyObjectRef { vm.ctx.new_int(v).into() };
                let key_obj = |k: KeyCode| -> PyObjectRef { PyKey(k).into_pyobject(vm) };
                let empty = || vm.ctx.new_str("").to_owned();
                match ev {
                    EngineEvent::Quit => Self {
                        ev, typ: tag("quit"), key: int_obj(0), x: 0, y: 0, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::KeyDown { key, modifiers, repeat } => Self {
                        ev, typ: tag("keydown"), key: key_obj(key), x: 0, y: 0, button: 0,
                        modifiers, repeat, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::KeyUp { key, modifiers } => Self {
                        ev, typ: tag("keyup"), key: key_obj(key), x: 0, y: 0, button: 0,
                        modifiers, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::TextInput { text } => {
                        // text 同时进载荷与展示槽——载荷侧克隆保全（String 非 Copy）
                        let ev = EngineEvent::TextInput { text: text.clone() };
                        Self {
                            ev, typ: tag("textinput"), key: int_obj(0), x: 0, y: 0, button: 0,
                            modifiers: 0, repeat: false,
                            text: vm.ctx.new_str(text).to_owned(), id: empty(), w: 0, h: 0,
                        }
                    }
                    EngineEvent::MouseMotion { x, y } => Self {
                        ev, typ: tag("mousemotion"), key: int_obj(0), x, y, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::MouseButtonDown { button, x, y } => Self {
                        ev, typ: tag("mousebuttondown"), key: int_obj(0), x, y,
                        button: i32::from(button), modifiers: 0, repeat: false,
                        text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::MouseButtonUp { button, x, y } => Self {
                        ev, typ: tag("mousebuttonup"), key: int_obj(0), x, y,
                        button: i32::from(button), modifiers: 0, repeat: false,
                        text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::MouseWheel { x, y } => Self {
                        ev, typ: tag("mousewheel"), key: int_obj(0), x, y, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::Resized { width, height } => Self {
                        ev, typ: tag("resized"), key: int_obj(0), x: 0, y: 0, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(),
                        w: width as i32, h: height as i32,
                    },
                    EngineEvent::FocusGained => Self {
                        ev, typ: tag("focusgained"), key: int_obj(0), x: 0, y: 0, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::FocusLost => Self {
                        ev, typ: tag("focuslost"), key: int_obj(0), x: 0, y: 0, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::Touch { phase: _, finger, x, y } => Self {
                        ev, typ: tag("touch"), key: int_obj(finger as u32), x, y, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::Suspend => Self {
                        ev, typ: tag("suspend"), key: int_obj(0), x: 0, y: 0, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::Resume => Self {
                        ev, typ: tag("resume"), key: int_obj(0), x: 0, y: 0, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::User { code } => Self {
                        ev, typ: tag("user"), key: int_obj(code), x: 0, y: 0, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    EngineEvent::Other => Self {
                        ev, typ: tag("other"), key: int_obj(0), x: 0, y: 0, button: 0,
                        modifiers: 0, repeat: false, text: empty(), id: empty(), w: 0, h: 0,
                    },
                    // 托盘交互（feature "tray" 产生路径；kind → typ tag，
                    // 菜单项 id 走 id 槽、按键走 button 槽、屏幕坐标走 x/y）
                    EngineEvent::Tray { kind, id, button, x, y } => {
                        let typ = match kind {
                            starfish_we::TrayKind::Menu => tag("traymenu"),
                            starfish_we::TrayKind::Click => tag("trayclick"),
                            starfish_we::TrayKind::DoubleClick => tag("traydoubleclick"),
                        };
                        // id 载荷侧克隆保全（String 非 Copy——TextInput 同款）
                        let ev = EngineEvent::Tray {
                            kind,
                            id: id.clone(),
                            button,
                            x,
                            y,
                        };
                        Self {
                            ev, typ, key: int_obj(0),
                            x: x as i32, y: y as i32, button: i32::from(button),
                            modifiers: 0, repeat: false,
                            text: empty(), id: vm.ctx.new_str(id).to_owned(), w: 0, h: 0,
                        }
                    }
                    // 文本对话框完成（Android IME 组合方案——脚本覆盖自己的
                    // 输入框内容；最终文本走 text 槽）
                    EngineEvent::TextDialogDone { text } => {
                        // 最终文本走 text 槽（与 textinput 同槽——脚本
                        // 以 type 区分语义）
                        let ev = EngineEvent::TextDialogDone {
                            text: text.clone(),
                        };
                        Self {
                            ev, typ: tag("textdialogdone"), key: int_obj(0), x: 0, y: 0,
                            button: 0, modifiers: 0, repeat: false,
                            text: vm.ctx.new_str(text).to_owned(), id: empty(), w: 0, h: 0,
                        }
                    }
                }
            }
        }

        /// 排空事件队列（引擎 event::pump 每帧灌入；引擎事件 → PyEvent
        /// 映射同源）。
        #[pyfunction]
        fn get(vm: &VirtualMachine) -> PyResult<PyObjectRef> {
            let items: Vec<PyObjectRef> = starfish_pygame::event::get()
                .into_iter()
                .map(|ev| PyEvent::from_engine(vm, ev).into_pyobject(vm))
                .collect();
            Ok(vm.ctx.new_list(items).into())
        }

        /// poll() -> Event | None：单发弹出队首（其余事件留在队列——
        /// 与 get() 的"全量排空"互补）。
        #[pyfunction]
        fn poll(vm: &VirtualMachine) -> PyResult<Option<PyObjectRef>> {
            Ok(starfish_pygame::event::poll()
                .map(|ev| PyEvent::from_engine(vm, ev).into_pyobject(vm)))
        }

        /// clear()：清空事件队列。
        #[pyfunction]
        fn clear(_vm: &VirtualMachine) {
            starfish_pygame::event::clear();
        }

        /// custom_code() -> int：自定义事件码分配（原子递增，进程内唯一
        /// ——`post_user` 的配对键：收到的 `e.type == "user"` 且
        /// `e.key == code`）。
        #[pyfunction]
        fn custom_code(_vm: &VirtualMachine) -> u32 {
            starfish_pygame::event::custom_code()
        }

        /// post_user(code)：用户事件入队（下次 get()/poll() 可收——
        /// typ = "user"，code 在 `e.key` 槽）。
        #[pyfunction]
        fn post_user(code: u32, _vm: &VirtualMachine) {
            starfish_pygame::event::post_user(code);
        }
    }


pub(crate) use event::{module_def,PyEvent};