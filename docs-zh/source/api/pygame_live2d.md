# pygame_live2d（已归档——独立库）

```{warning}
⚠️ **使用前必读**：本模块**编译默认不含**——构建须显式激活
`features = ["live2d"]`，否则 `import pygame_live2d` 即 ImportError。
法律风险说明见下方归档声明。
```

> **归档声明**（2026-11-06）：Live2D/Cubism 因法律风险归档为**独立库**
> [pygame_live2d](https://github.com/GuYeying/pygame_live2d)——实现、
> 渲染管线、文档随库维护，本仓仅存 feature 门控的薄绑定
> （`features = ["live2d"]`），不再随 pygame-rs 主线维护。
>
> **API 文档随库**：见 pygame_live2d 仓 `docs/API.md`（权威来源）。
>
> 速览：`import pygame_live2d` 直达；`load_model / load_runtime /
> load_motion / motion_player`；sprite 工作流 =
> `rt.draw(surface, x=0, y=0, scale=0.9)` 全管线自理（蒙版纹理方案
> GPU 化）→ `pygame.gfxdraw.draw_surface` 上屏。资产（Hiyori）与
> demo（live2d_render_demo 等）见独立仓与 pygamest-dev examples。
