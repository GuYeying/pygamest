# Windows 宿主

## 获取

从仓库发布页或构建产物获取 `python.exe` 宿主（50MB 单文件，RustPython
解释器 + 引擎 + 绑定全内置）。

## 运行案例

```bash
cd pygamest-dev/examples
..\target\release\python.exe hello_world.py
```

- 入口恒为 `async def main()`（跨端统一约束——Web rAF 模型的硬要求）
- `ESC` / 关窗退出
- 案例 list 见 [示例代码](../examples/index.md)

## 常用案例速查

| 案例 | 内容 |
|---|---|
| `hello_world.py` | 最小起步 |
| `cube_rotate.py` | 自定义顶点布局 + 纹理立方体 |
| `compute_probe.py` | GPU 粒子系统（计算着色器，105 万粒档位） |
| `window_mode_test.py` | 显示模式运行时切换 |
| `io_test.py` / `net_test.py` / `video_test.py` | 三端一致性模块案例 |

## 诊断开关

```bash
# 内存采样（3s 一拍：live_alloc / live_bytes / total_alloc / rss）
set PYGAMEST_MEMTRACE=1
python.exe font_test.py
```

方法论见仓库 `reference/如何调试寻找内存泄漏.md`。
