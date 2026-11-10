# pygame.display

窗口与显示面——`set_mode()` 开窗（返回 `Screen` 绘制目标）、`flip()`
换帧收尾。窗口旗标创建期收下、运行时生效（`RESIZABLE` / `FULLSCREEN` /
`NOFRAME` 即时可切换）。

## <span class="c-const">常量</span>

窗口旗标位（`set_mode` 的 `flags` 参数；收 int 位掩码或 `[位, ...]`
序列两种形态）。

| 旗标 | 语义 |
|---|---|
| RESIZABLE | 可拖边缩放（默认已带边框）|
| FULLSCREEN | 全屏铺满显示器 |
| NOFRAME | 无边框 |
| HIDDEN | 创建期隐藏窗口 |
| ALWAYS_ON_TOP | 置顶（桌宠形态）|
| TRANSPARENT | 透明窗口（alpha 0 区域透出桌面）|
| VULKAN / DIRECTX / METAL / OPENGLES | 渲染后端声明（前向兼容，收下不解析）|

## <span class="c-func">函数</span>

#### <span class="c-func">set_mode(size, flags=0) -> Screen</span>

开窗并返回 Screen 绘制目标。

```python
screen = pygame.display.set_mode((800, 600))
screen = pygame.display.set_mode(
    (0, 0), [pygame.display.FULLSCREEN, pygame.display.RESIZABLE])
```

- :param size: 逻辑尺寸
- :type size: tuple[int, int]
- :param flags: 旗标位集
- :type flags: int | list[int]
- :returns: Screen 绘制目标
- :rtype: Screen

## <span class="c-class">Screen —— 绘制终点</span>

`set_mode` 返回的 Screen 对象（真身 = Rust 全局 Screen——宿主壳装配）。

#### <span class="c-func">screen.render()</span>

with 会话入口：进入 = 开启 RenderPass，退出 = finish 打包
（`with screen.render() as f:`——绘制走 [pygame.gfxdraw](gfxdraw.md) /
[pygame.font](font.md)，会话对象只做目标表达）。

#### <span class="c-func">screen.set_camera(cam)</span>

**显式设置渲染相机**（2026-11 定约——渲染会话必传，杜绝兜底）。
`cam` = `pygame.render.game_camera_ortho(lw, lh, pw, ph)` /
`game_camera_perspective(fov, lw, lh, pw, ph, near, far)` 构造的
GameCamera（letterbox 内聚——MVP / 坐标映射 / 裁剪同一事实源，详见
[pygame.render](render.md)）。设置后 set_mode / resize 的自动重建不再
覆盖（显式优先于自动）；物理视口变化走 `cam.resized(pw, ph)`。
构造入参 pw/ph = 设计意图——**set 时引擎以当前物理表面对账
letterbox**（投影/视图/逻辑面不动；跨设备可移植，桌面恰好相等）。

```python
cam = pygame.render.game_camera_perspective(58.0, 800, 600, 800, 600, 0.1, 300.0)
screen.set_camera(cam)
```

#### <span class="c-func">screen.get_size() -> tuple[int, int]</span>

当前逻辑尺寸（Web 响应式 canvas 下跟随页面布局变化——脚本查询用）。

- :rtype: tuple[int, int]

#### <span class="c-func">flip()</span>

帧收尾——把本帧绘制结果呈现到屏幕（主循环每帧末尾调用）。

#### <span class="c-func">set_opacity(v)</span>

窗口整体不透明度（`0.0` 透明 .. `1.0` 不透明）——桌宠/悬浮件形态。
与 `TRANSPARENT` 创建旗标正交：后者让 alpha 0 区域透出桌面，前者调
窗口整体 alpha。

#### <span class="c-func">set_caption(title)</span>

窗口标题。

#### <span class="c-func">get_init() -> bool</span>

display 初始化态（`set_mode` 是否已调用）。

- :rtype: bool

#### <span class="c-func">quit()</span>

关闭显示（窗口/屏句柄回收——pygame 标准对；`get_init` 归 False，
可再 `set_mode` 重开）。

#### <span class="c-func">map_point(x, y) -> (lx, ly)</span>

物理点 → **逻辑中心系**点（呈现映射逆变换——letterbox 等比缩放 +
居中 + y 翻转 + 中心原点化）。脚本自处理原始触点/外部坐标时用；
泵层事件坐标已自动映射（常规事件流无需此口）。display 未就绪 =
原样透传。

- :rtype: tuple[float, float]

## <span class="c-func">桌宠能力位</span>

三组窗口系统能力开关（桌宠/悬浮件形态配套）——全部
**feature 门控**：未启用对应 feature 的构建无这些 API（AttributeError
= 编译期即知不支持）；启用后不支持的平台运行时 Err（站桩原则）。

| 能力 | feature | Windows | macOS | Linux | web/mobile |
|---|---|---|---|---|---|
| 鼠标穿透 | click-through | ✅ | 站桩 | 站桩 | Err |
| 任务栏隐藏 | skip-taskbar | ✅ | ✅ Dock 级 | 站桩 | Err |
| 系统托盘 | tray | ✅ | ✅ | ✅ ksni | Err |

#### <span class="c-func">set_click_through(enable)</span>

鼠标事件穿透开关（点击落到下层窗口）。⚠️ 穿透后窗口收不到任何
鼠标事件——恢复须脚本逻辑/外部通道，防自锁。

- :param enable: True = 穿透开
- :type enable: bool

#### <span class="c-func">get_click_through() -> bool</span>

窗口当前是否鼠标穿透（OS 位读回）。

#### <span class="c-func">set_skip_taskbar(enable)</span>

任务栏（Dock）隐藏开关。Windows = `WS_EX_TOOLWINDOW` 位切换
（⚠️ Alt+Tab 一并隐藏——工具窗语义，桌宠通常正合预期）；macOS =
Dock 级 `setActivationPolicy(.accessory/.regular)`（app 整体隐藏/
恢复 Dock 图标——macOS 无 per-window 任务栏概念）；Linux =
Wayland 标准协议不控制任务栏（站桩 Err）。

- :param enable: True = 任务栏隐藏
- :type enable: bool

#### <span class="c-func">get_skip_taskbar() -> bool</span>

窗口当前是否任务栏隐藏（OS 状态读回）。

#### <span class="c-func">set_tray(icon, tooltip="", menu=None)</span>

安装（或替换）系统托盘图标：RGBA 图标 + 悬停提示 + 平铺菜单。
交互经事件上浮：左键点图标 = `trayclick`、右键 = 弹菜单、菜单项
点击 = `traymenu`（见 [pygame.event](event.md)）；托盘图标双击 =
`traydoubleclick`（Windows）。Linux 后端 = ksni 纯 D-Bus（Ubuntu
自带 AppIndicator 扩展可显示；其他 GNOME 需装扩展）。

```python
icon = pygame.Surface((64, 64))
with icon.render() as t:
    t.fill((40, 120, 60))
pygame.display.set_tray(icon, "我的桌宠",
                        [("show", "显示"), ("quit", "退出")])
```

- :param icon: 离屏 Surface（**Screen 背板不可作图标**——不回读）
- :param tooltip: 悬停提示（缺省无）
- :param menu: `[(id, label), ...]` 平铺菜单（缺省/空表 = 纯图标）

#### <span class="c-func">remove_tray()</span>

移除托盘图标（未安装时空操作）。
