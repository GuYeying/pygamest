# pygamest

**次世代 pygame —— Python 游戏开发的原生形态。**

[starfish](https://github.com/GuYeying/starfish) 引擎（wgpu GPU 渲染）的
Python 绑定层：内嵌 [RustPython](https://github.com/RustPython/RustPython)
解释器，`import pygame` 直达 **Rust 原生模块**——pygame 的心智，GPU 的
出生面，三平台一套代码。

## 快速开始

开发用 [pygamest-dev](https://github.com/GuYeying/pygamest-dev) 解释器，
分发用 [pygamest-runtime](https://github.com/GuYeying/pygamest-runtime)。

```python
import pygame

async def main():
    screen = pygame.display.set_mode((800, 600))
    image  = await pygame.image.load("assets/ferris.png")

    while True:
        for e in pygame.event.get():
            if e.type == pygame.event.QUIT:
                return
        with screen.render() as r:
            r.blit(image, (0, 0))
        pygame.display.flip()
        await pygame.next_frame()
```


## 徽章图例

| 徽章 | 含义 |
|---|---|
| <span class="bd bd-magic">魔术方法</span> | `__init__` / `__repr__` / `__eq__` 等 |
| <span class="bd bd-strict">严格私有</span> | `__attr` / `__method`（双下划线）|
| <span class="bd bd-soft">约定私有</span> | `_attr` / `_method`（单下划线，虚线边框区分严格度）|
| <span class="bd bd-ok">普通</span> | 属性 / 方法 / 模块函数 |
| <span class="bd bd-static">静态方法</span> | `@staticmethod` |
| <span class="bd bd-ok">函数</span><span class="bd bd-async">async</span> | **异步叠加标记**（虚线描边——async 是修饰符非类型，任意徽章可叠加）|

异步 API 在导航中以 <a class="async-link">紫色链接 + ↻</a> 标识。

## 文档导航

- [pygame.event —— 事件队列与事件对象](event.md)

## 与 pygame 的关系声明

pygamest 是**独立项目**：与 pygame 官方（pygame.org / pygame-ce）无隶属、
无关联、无背书，不使用其任何代码、商标或美术资源。API 形态保持心智兼容
是降低学习成本的工程选择。

## 协议

MIT OR Apache-2.0 双协议（见 LICENSE-MIT / LICENSE-APACHE）。

> 本软件按"现状"提供，不含任何形式的担保；使用造成的任何损失由使用者
> 自负——完整条款见 LICENSE 文本。
