# 示例代码

全部案例三端同一脚本（Windows 宿主 / WASM / Android APK 内置），按
模块分组。运行方式见 [安装](../install/index.md)。

## 入门

| 案例 | 内容 |
|---|---|
| `hello_world.py` | 最小起步——会话 + fill + flip |
| `center_origin_demo.py` | 中心原点坐标系（2D + 3D 同屏对照） |

## 模块三端一致性案例（断言驱动，SUMMARY 收官）

| 案例 | 模块 |
|---|---|
| `io_test.py` | pygame.io（同步 + 异步通道） |
| `net_test.py` | pygame.net（TCP 分帧回声 + UDP 自环；需回声服） |
| `video_test.py` | pygame.video（BBB 测试片全链断言） |
| `font_test.py` | pygame.font（字体全链） |
| `draw_test.py` | pygame.draw 六形状 + 半透明 |
| `input_test.py` | key / mouse / event 输入三件套 |
| `window_mode_test.py` | 显示模式运行时切换（单选四模式——手机测试定向） |
| `touch_test.py` | 触摸（手机定向） |
| `time_test.py` | Clock.tick 节流 + delay |
| `audio_test.py` | mixer 程序化 WAV（零资产依赖） |
| `gamepad_test.py` | 手柄（未连接 = 合法输出） |
| `image_test.py` | image load / scale / flip / rotate |
| `math_test.py` | Mat4 / quat / 三角函数断言驱动 |

## 渲染进阶

| 案例 | 内容 |
|---|---|
| `cube_rotate.py` | 自定义顶点布局 + 纹理立方体（mesh 任意布局） |
| `mesh_builder_demo.py` | 任意顶点布局 + 程序化棋盘 UV |
| `custom_pipeline_demo.py` | 自建 WGSL 管线（raw_render / raw_compute 双通道） |
| `compute_probe.py` | GPU 粒子系统（档位可选至 105 万粒） |
| `gfx3d_demo.py` | gfxdraw 3D 词汇（深度会话 + 透视相机） |
| `pip_test.py` | 画中画 / 游戏内面板 |
| `blit_transform_demo.py` | blit_transform 空间放置（近大远小） |
| `render_unify_demo.py` | per-draw 相机/变换状态机全形态 |

## 动画三库

| 案例 | 库 |
|---|---|
| `spine_render_demo.py` / `spine_control_test.py` / `spine_full_test.py` | spine（chine） |
| `ozz_render_demo.py` / `ozz_full_test.py` | ozz |
| `live2d_render_demo.py` / `live2d_full_test.py` | Live2D |
| `anim3_wasm_test.py` | 三库同屏（wasm） |

## 物理 / GUI / 综合

| 案例 | 内容 |
|---|---|
| `rapier_test.py` / `rapier3d_test.py` | rapier 2D/3D 物理 |
| `egui_test.py` | egui 即时 GUI 全面对接 |
| `ime_egui_demo.py` | Android IME + egui 输入框 |
| `net_egui_chat.py` | egui + net 综合案例 |
| `window_mode_test.py` | 显示模式单选（手机定向） |

## 桌宠能力位

| 案例 | 内容 |
|---|---|
| `click_through_test.py` | 鼠标穿透 |
| `skip_taskbar_test.py` | 任务栏隐藏 |
| `tray_test.py` | 系统托盘 |

## 泄漏回归套件（leak_a ~ leak_i）

隔离化内存探针——每支隔离一类通路（新纹理 / 同纹理 / compute / egui /
双写 / 渲染 / Font(None) / 相机管线 / gfx3d），配 `leak_io_net_video`
复合探针。方法论见仓库 `reference/如何调试寻找内存泄漏.md`。
