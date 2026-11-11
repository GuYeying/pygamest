# 源代码架构与性能

## 三层架构

```
pygamest（绑定层）—— RustPython 嵌入 + pygame.* 词汇对接
  └── starfish-pygame（引擎层）—— 渲染会话 / batch / 绘制词汇 / 相机
        ├── starfish-we —— 窗口/事件/平台（六平台后端制）
        ├── starfish-render —— wgpu 封装（材质/管线/表面）
        ├── starfish-gfx —— 几何生成器（shape2d/shape3d/tri）
        └── starfish-io/net/video/audio/font —— 平台模块族
```

原则：**starfish-pygame 完成实现，pygamest 纯对接**；平台差异收敛在
各平台模块内（窗口后端 / 视频后端 / 音频 device trait），核心层零
平台字符串。

## 性能要点（已知债务与优化位）

| 项 | 状态 |
|---|---|
| raw_render 管线缓存 | ✅ with 块内 set_pipeline/set_bind_group 指针去重 |
| 每帧内联一次性 vertex buffer | P3 优化位——append-only arena + slice 偏移（无上限 fps 下缓冲创建速率超驱动回收 → OOM；demo 帧率 delay 规避） |
| bind 缓存孤儿条目 | ✅ 已根治（Drop 入队 pending_unbinds + encode 尾 drain） |

## 内存泄漏排查

方法论文档：仓库 `reference/如何调试寻找内存泄漏.md`。隔离化探针套件
`examples/leak_a ~ leak_i`（每支隔离一类通路）+ 复合探针
`leak_io_net_video`；`PYGAMEST_MEMTRACE=1` 开启 3s 一拍的
live_alloc / live_bytes / total_alloc / rss 采样。
