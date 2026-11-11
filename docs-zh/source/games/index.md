# 用 pygamest 做的游戏与交互案例

精选可玩/可交互的案例（全部三端同源）：

| 案例 | 玩点 |
|---|---|
| `compute_probe.py` | GPU 粒子系统——档位切到 105 万粒（egui 档位面板 + FPS 实时） |
| `pip_test.py` | 画中画 / 游戏内面板（Surface 嵌套渲染） |
| `net_egui_chat.py` | 局域网聊天（egui 面板 + net TCP） |
| `cube_rotate.py` | 纹理立方体旋转（自定义顶点布局 + cloth 贴图） |
| `gfx3d_demo.py` | 3D 场景摆件（立方体/球/圆柱/圆锥/六边形板） |
| `window_mode_test.py` | 显示模式即时切换（手机横竖屏测试） |

把你的游戏做完后：走 [分发](../distribute/index.md) 三端打包链
（Windows exe / 网页 / APK），分发物零明文零脚本后缀。
