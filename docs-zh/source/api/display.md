# pygame.display

窗口与显示模块——创建游戏窗口、控制呈现、管理窗口属性。

```python
import pygame

screen = pygame.display.set_mode((800, 600))

while True:
    # ...绘制...
    pygame.display.flip()
```

## 常量

窗口旗标（`set_mode` 的 `flags` 参数——接受 int 位掩码或
`[旗标, ...]` 列表两种形式）。

| 旗标 | 语义 |
|---|---|
| RESIZABLE | 可拖边缩放（默认已带边框）|
| FULLSCREEN | 全屏铺满显示器 |
| NOFRAME | 无边框 |
| HIDDEN | 创建期隐藏窗口 |
| ALWAYS_ON_TOP | 置顶（桌宠形态）|
| TRANSPARENT | 透明窗口（alpha 0 区域透出桌面）|

注意：渲染后端的选择不在窗口旗标里——通过
[GpuSettings.backends](render.md) 配置。

## 函数

### set_mode(size, flags=0, surface_settings=None, gpu_settings=None) -> Screen

创建游戏窗口，返回 `Screen` 绘制目标。

```python
screen = pygame.display.set_mode((800, 600))

# 全屏 + 可缩放
screen = pygame.display.set_mode(
    (0, 0), [pygame.display.FULLSCREEN, pygame.display.RESIZABLE])

# 省内存形态（运存优化）
gpu = pygame.render.GpuSettings(memory_hints="memory_usage",
                                power_preference="low_power")
surf = pygame.render.SurfaceSettings(desired_maximum_frame_latency=1)
screen = pygame.display.set_mode((800, 600), gpu_settings=gpu,
                                 surface_settings=surf)
```

- size —— 逻辑尺寸；`(0, 0)` 配 FULLSCREEN 表示使用显示器原始分辨率
- flags —— 窗口旗标（见常量表）
- surface_settings —— 表面/交换链配置
  （[pygame.render.SurfaceSettings](render.md#surfacesettings-表面交换链配置)——
  仅首次 GPU 装配时消费）
- gpu_settings —— GPU 装配配置
  （[pygame.render.GpuSettings](render.md#gpusettings-gpu-装配配置)）
- 返回 `Screen` 对象

**平台装配差异**：桌面端在本调用内完成 GPU 装配；Web 端无阻塞模型，
本调用只发起窗口请求（配置走构建期通道）；Android 在引擎门面先行
装配。重复调用 `set_mode` 不会重新装配 GPU，仅按请求调整窗口。

**调用顺序**：依赖 GPU 的调用（`egui.init()`、`render()`、
`render.access()` 等）必须在 `set_mode` 之后——顺序错误会收到明确的
运行时错误提示。

### flip()

将本帧绘制结果呈现到屏幕。主循环每帧末尾调用一次：

```python
while True:
    # ...绘制...
    pygame.display.flip()
```

### get_init() -> bool

返回 display 是否已初始化（`set_mode` 是否已调用）。

### quit()

关闭显示，回收窗口资源。之后可再次 `set_mode` 重开。

### set_caption(title)

设置窗口标题文字。

```python
pygame.display.set_caption("我的游戏")
```

### set_opacity(opacity)

设置窗口整体不透明度，`0.0`（全透明）到 `1.0`（不透明）。
与 `TRANSPARENT` 创建旗标正交：后者让 alpha 0 区域透出桌面，
前者调整整个窗口的不透明度。

### set_orientation(orientation)

请求屏幕方向。取值：`"portrait"` / `"landscape"` /
`"sensor_portrait"` / `"sensor_landscape"` / `"sensor"` /
`"full_sensor"` / `"unspecified"`（语义同 Android ActivityInfo 常量）。

```python
pygame.display.set_orientation("sensor_portrait")   # 竖屏应用
```

Android 上即时生效（运行时覆盖清单文件的初始方向设置）；
桌面与 Web 平台为空操作（跨平台脚本无需分支判断）。

### map_point(x, y) -> (lx, ly)

把物理像素坐标转换为逻辑中心系坐标（原点屏幕中心、y 向上）。
处理原始触点或外部坐标输入时使用——常规事件流中的坐标已自动映射，
无需调用。

## 多窗口（仅 Windows）

在主窗口之外创建额外的渲染窗口。需要构建时启用 `multi-window`
feature（Linux / macOS / 移动 / Web 构建不支持）；运行时同样校验，
不支持时 `create_window` 抛 `RuntimeError`。

#### create_window(size, flags=0, id="win2") -> Screen

创建副窗口并返回新的 `Screen` 绘制目标（独立的 `render()` 与
`flip()` 会话）。

- id —— 窗口标识；`"main"` 为主窗保留，id 冲突抛错
- 副窗支持 render / fill / flip / get_size

```python
win2 = pygame.display.create_window((400, 300), id="win2")
with win2.render() as r:
    r.fill((30, 30, 40))
win2.flip()
```

## Screen 对象

`set_mode` 与 `create_window` 返回的绘制目标。

#### render()

with 会话入口——进入时开启渲染通道，退出时打包提交本帧绘制。
绘制命令走 [pygame.gfxdraw](gfxdraw.md)、[pygame.font](font.md) 等
模块：

```python
with screen.render() as r:
    r.fill((28, 34, 46))
    pygame.gfxdraw.box(r, (10, 10, 100, 50))
```

#### set_camera(cam)

设置渲染相机（世界 → 画布）。相机由
`pygame.render.game_camera_ortho()` / `game_camera_perspective()`
构造。一次设置后跨会话与窗口缩放缓存，未显式设置时渲染走默认正交域。

```python
cam = pygame.render.game_camera_perspective(
    58.0, 800, 600, 800, 600, 0.1, 300.0)
screen.set_camera(cam)
```

#### get_size() -> (width, height)

返回当前逻辑尺寸（Web 响应式画布下跟随页面布局变化）。

#### raw_render(depth=False) -> RawPass

底层直绘会话——`with screen.raw_render() as p:` 记录自定义管线命令
（[pygame.render](render.md)），帧收尾时回放，绘制在常规内容之上。

```python
with screen.raw_render(depth=True) as p:
    p.set_pipeline(voxel_pipe)
    p.set_bind_group(0, voxel_bind)
    p.draw_mesh(voxel_mesh)
```

#### raw_compute() -> RawCompute

计算管线会话（独立 compute pass，帧收尾回放）。

## 桌宠能力位

三组窗口系统能力开关（桌宠 / 悬浮件形态配套）。全部为构建期 feature
门控：未启用对应 feature 的构建没有这些 API；启用后不支持的平台
运行时抛错。

| 能力 | feature | Windows | macOS | Linux | web / 移动 |
|---|---|---|---|---|---|
| 鼠标穿透 | click-through | ✅ | 空操作 | 空操作 | 抛错 |
| 任务栏隐藏 | skip-taskbar | ✅ | ✅ Dock 级 | 空操作 | 抛错 |
| 系统托盘 | tray | ✅ | ✅ | ✅ ksni | 抛错 |

#### set_click_through(enable)

鼠标事件穿透开关——开启后点击落到下层窗口。注意：穿透后本窗口收
不到任何鼠标事件，恢复需要脚本逻辑或外部通道。

#### get_click_through() -> bool

返回当前是否处于鼠标穿透状态。

#### set_skip_taskbar(enable)

任务栏 / Dock 隐藏开关。Windows 切换工具窗位（Alt+Tab 一并隐藏）；
macOS 使用 Dock 级策略切换。

#### get_skip_taskbar() -> bool

返回当前是否任务栏隐藏。

#### set_tray(icon, tooltip="", menu=None)

安装（或替换）系统托盘图标。交互事件经事件系统上浮：左键点击图标 =
`trayclick`、右键 = 弹出菜单、菜单项点击 = `traymenu`、双击 =
`traydoubleclick`（Windows）。

```python
icon = pygame.Surface((64, 64))
with icon.render() as t:
    t.fill((40, 120, 60))
pygame.display.set_tray(icon, "我的桌宠",
                        [("show", "显示"), ("quit", "退出")])
```

- icon —— 离屏 Surface 图标
- tooltip —— 悬停提示文字（缺省无）
- menu —— `[(id, label), ...]` 平铺菜单（缺省或空表 = 纯图标）

#### remove_tray()

移除托盘图标。未安装时为空操作。
