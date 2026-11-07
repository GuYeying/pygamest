## Context

桌宠形态第二梯队能力：**skip-taskbar**（隐藏任务栏/Dock 图标）+ **tray**（系统托盘图标+菜单）。架构完全复刻 click-through M1 定约：starfish-we feature 门控 → starfish-pygame → pygamest(pygame-rs) → pygamest-dev 四层纯透传；feature 未启用 = API 不存在（编译期即知），启用后不支持平台 = 运行时 Err 站桩。

**平台支持矩阵（用户定约：Windows + macOS + Ubuntu 实装，其余站桩）**：
| 平台 | skip-taskbar | tray |
|---|---|---|
| Windows | ✅ WS_EX_TOOLWINDOW 位切换 | ✅ tray-icon Windows 后端 |
| macOS | ✅ Dock 级 setActivationPolicy(.accessory/.regular 切换) | ✅ tray-icon objc2 后端（主线程=窗口线程 ✓） |
| Linux | ⬜ 站桩 Err（Wayland 无任务栏协议；X11 后端建成后可补） | ✅ tray-icon ksni 纯 D-Bus 后端（免 GTK 依赖，Ubuntu 自带 AppIndicator 扩展） |
| web/iOS/Android | ⬜ 站桩 Err | ⬜ 站桩 Err |

**侵入面（纯增量，既有接口零改动）**：唯一触碰既有代码的两处——①`Event` 枚举加 1 变体（`#[non_exhaustive]`，通配臂 match 不受影响）；②windows `poll_event` 泵后追加 cfg 门控 drain 块（feature 关 = 编译期剔除，行为逐位不变）。其余全部是新文件/新增 cfg 方法/Cargo 追加。

关键侦察结论：
- starfish-we **自研窗口后端**（无 winit/tao），`poll_event` 内 `PeekMessageW(null hwnd)` 泵全线程消息——tray-icon 的隐藏窗口消息被同线程泵自然派发，事件经 `TrayIconEvent::receiver()`/`MenuEvent::receiver()` crossbeam channel 轮询取出
- tray-icon 0.26 运行时依赖 muda ^0.21 并再导出为 `tray_icon::menu`——**不单独直依赖 muda**（避免双版本 MenuEvent receiver 错位；对用户原话 `dep:muda` 的偏离点）
- Surface→RGBA 回读现成：starfish-pygame `image.rs::readback_rgba`（pub(crate)，image.save 同款）

## 实施步骤

### 1. MSRV 校验
`rustc --version` ≥ 1.90 则 tray-icon 0.26.x；否则降 pin（0.2x 系 API 同型）。

### 2. starfish-we（基座，`D:\Projects\Rust\starfish\crates\starfish-we`）
- **Cargo.toml**：
  ```toml
  # 任务栏/Dock 隐藏（桌宠配套——Win=WS_EX_TOOLWINDOW 位切换、
  # macOS=Dock 级 setActivationPolicy；Linux Wayland 无协议站桩 Err）
  skip-taskbar = []
  # 系统托盘（tray-icon 三桌面通吃——Win/macOS 默认后端、Linux ksni
  # 纯 D-Bus；muda 经 tray_icon::menu 再导出，不单独直依赖防双版本）
  tray = ["dep:tray-icon"]
  ```
  ```toml
  [target.'cfg(target_os = "windows")'.dependencies]
  tray-icon = { version = "0.26", optional = true }
  [target.'cfg(target_os = "macos")'.dependencies]
  tray-icon = { version = "0.26", optional = true }
  [target.'cfg(target_os = "linux")'.dependencies]
  tray-icon = { version = "0.26", optional = true, default-features = false, features = ["ksni"] }
  ```
- **src/tray.rs**（新，cfg(feature="tray")）：`TraySpec { icon_rgba: Vec<u8>, icon_w/icon_h: u32, tooltip: String, menu: Vec<(String, String)> }`（(id, label) 平铺菜单）+ `TrayKind { Menu, Click, DoubleClick }` + `TrayEvent { kind, id, x, y }` + 跨平台共用的 `drain_events() -> Vec<TrayEvent>`（两个 receiver try_iter）——window.rs 重新导出
- **src/event.rs**：`Event::Tray(TrayEvent)` 变体（无条件，Touch 先例）
- **src/window.rs** 门面（照抄 click-through cfg 方法形态）：
  - `#[cfg(feature = "skip-taskbar")] set_skip_taskbar(bool) -> Result<(), WindowError>` / `is_skip_taskbar() -> bool`
  - `#[cfg(feature = "tray")] set_tray(TraySpec) -> Result<(), WindowError>` / `remove_tray() -> Result<(), WindowError>`
- **src/platform/windows/mod.rs**：
  - skip-taskbar：`GetWindowLongW/SetWindowLongW` GWL_EXSTYLE `WS_EX_TOOLWINDOW` 位切换（click-through 同款习语，OS 位读回式查询）+ `SetWindowPos(SWP_FRAMECHANGED)` 补通知
  - tray：`thread_local RefCell<Option<TrayIcon>>`；`Icon::from_rgba` + `TrayIconBuilder::with_icon/with_tooltip/with_menu`（左键=事件、右键=菜单）；remove = take；poll_event 泵后调 `crate::tray::drain_events()` → `Event::Tray` 入 pending
- **src/platform/macos/mod.rs**：
  - skip-taskbar：objc2-app-kit `NSApplication::sharedApplication().setActivationPolicy(Accessory/Regular)`（Dock 级；读回 = activationPolicy()==Accessory）
  - tray：同 windows 形态（thread_local TrayIcon；主线程 = 窗口线程 ✓）；poll_event 泵后 drain
- **src/platform/linux/mod.rs**：
  - tray：同形态（ksni 后端内部自带 worker 线程，事件仍走 receiver channel）；poll_event 泵后 drain
  - skip-taskbar：站桩 Err——"Wayland 标准协议不控制任务栏，X11 后端建成后可补"
- **src/platform/{web,ios,android}/mod.rs**：4 方法全站桩 Err + is_ 恒 false（照抄 click-through 文案模式）

### 3. starfish-pygame（兼容层）
- **Cargo.toml**：`skip-taskbar = ["starfish-we/skip-taskbar"]`；`tray = ["starfish-we/tray"]`
- **src/event.rs**：`Event` 枚举加 `Tray { kind: starfish_we::TrayKind, id: String, x: f64, y: f64 }`（无条件变体）；`translate` 加分支
- **src/display.rs**（照抄 set_click_through 三态 with_window 形态）：
  - `#[cfg(feature = "skip-taskbar")] set_skip_taskbar(bool) -> Result<(), String>` / `get_skip_taskbar() -> bool`
  - `#[cfg(feature = "tray")] set_tray(surface: &Surface, tooltip: &str, menu: &[(String, String)]) -> Result<(), String>`——`block_on(readback_rgba(surface))` 拿 RGBA（image::save 同款）再 with_window；`remove_tray() -> Result<(), String>`

### 4. pygamest = pygame-rs（绑定层）
- **Cargo.toml**：`skip-taskbar = ["starfish-pygame/skip-taskbar"]`；`tray = ["starfish-pygame/tray"]`
- **src/rpy/pygame/event.rs**：PyEvent 加 `id` 槽（PyStrRef 默认空串；既有 from_engine 各臂补默认值）；`from_engine` 加 `EngineEvent::Tray` 分支（kind→typ tag）；常量 `#[pyattr] TRAYMENU="traymenu"` / `TRAYCLICK="trayclick"` / `TRAYDOUBLECLICK="traydoubleclick"`
- **src/rpy/pygame/display.rs**（照抄 set_click_through 的 `#[cfg] #[pyfunction]` 形态）：
  - `#[cfg(feature = "skip-taskbar")] set_skip_taskbar(enable)` / `get_skip_taskbar() -> bool`
  - `#[cfg(feature = "tray")] set_tray(icon: PyRef<PySurface>, tooltip: Option<str>, menu: Option<Vec<(String, String)>>)`（FromArgs 收参）；`remove_tray()`

### 5. pygamest-dev = pygame-rs-dev（宿主）
- **Cargo.toml**：`skip-taskbar = ["pygamest/skip-taskbar"]`；`tray = ["pygamest/tray"]`（不入 default，与 click-through 同待遇）

### 6. 测试（pygame-rs-dev/examples）
- `skip_taskbar_test.py`：照抄 click_through_test.py——set_mode → set_skip_taskbar(True) → 读回 assert → 600 帧每 300 帧翻转 + 每帧读回 assert（Windows 视觉验收：任务栏按钮消失，手动）
- `tray_test.py`：set_mode → 程序化 Surface 画图标 → set_tray(surface, "pygamest", menu=[("show","显示"),("quit","退出")]) → 循环 get() 打印 traymenu/trayclick 事件 → remove_tray + 重装验证 → 自动退出（菜单点击手动验收）

### 7. 文档 + 进度
- docs-zh display 页补四口 + 平台支持矩阵（Ubuntu 需 AppIndicator 扩展、macOS Dock 级语义、Linux skip-taskbar Wayland 限制）
- reference/收尾目标进度跟踪.md 追加记录

## 关键设计决策（已定）
1. **不带 dep:muda 直依赖**——tray_icon::menu 再导出全量菜单 API，防双版本 receiver 错位
2. **skip-taskbar macOS = Dock 级**（app 整体 activation policy——macOS 无 per-window 任务栏概念）
3. **skip-taskbar Linux 站桩**——Wayland 无标准协议；诚实文案给后续路径
4. **Linux tray = ksni 后端**——纯 D-Bus 免 GTK 系统依赖
5. **图标源 = Surface**（readback_rgba 现成；路径形态不需要——pygame.image.load 已闭环）
6. **事件枚举变体无条件存在**（Touch/Suspend 先例），feature 只门控产生路径与 API 面

## 验证
1. `cargo check -p starfish-we --features "skip-taskbar,tray"`（Windows 本机）
2. **跨目标编译门**：`cargo check --target x86_64-unknown-linux-gnu` / `--target aarch64-apple-darwin`（check 不链接——验证 macos/linux 实装编译正确性）
3. pygame-rs-dev `cargo build --features "skip-taskbar,tray"`
4. `python.exe examples/skip_taskbar_test.py`（读回断言全绿）+ `tray_test.py`（Err-free + 手动点托盘看菜单/收事件）
5. 默认构建回归（不含新 feature 不受影响）
6. macOS/Linux 运行时验收留对应宿主（本机为 Windows）
