# 社区

## 仓库

| 仓 | 内容 |
|---|---|
| [pygamest](https://github.com/GuYeying/pygamest) | 绑定 + RustPython 嵌入（本文档） |
| [starfish](https://github.com/GuYeying/starfish) | 引擎族（we/render/gfx/io/net/video/audio…） |
| [pygamest-dev](https://github.com/GuYeying/pygamest-dev) | 开发宿主 + 案例目录 |
| [pygamest-runtime](https://github.com/GuYeying/pygamest-runtime) | 生产宿主 + 三端打包链 |

## 问题反馈

提 issue 时附：平台（Windows / Android / WASM）、复现脚本（examples
目录任一案例改造最小复现最佳）、控制台输出（`[ERROR]` 行与
traceback）。

## 贡献

- 案例：放 `examples/`，入口 `async def main()`，断言驱动（SUMMARY
  收官）优先——参考 `io_test.py` 形态
- 文档：`docs-zh/source/`（Sphinx + MyST）
- 引擎：starfish 仓各 crate，平台差异收敛在平台模块内
