# pygame.io

资源与存档读写——**跨平台统一走向**（v3.1，2026-11-20）：native 同步
面全量可用；**Web 端同步口退化为同步 XHR 全量**（`read`/`read_text`/
`exists` 不再 Err——阻塞语义与 native 对称，2026-11-19 用户定约）；
**大文件流式加载**走 `fetch`（native = 分块循环每帧一块交出控制权，
wasm = 浏览器异步全量）。

```{note}
[基准目录] Python 面恒以 **main 所在目录**为基准写相对路径——打包形态
不改写调用写法（Android 基准目录由引擎 run 自动注入，`set_base_dir`
不暴露）。

[走向矩阵] `read`/`read_text`/`exists`/`write`/`write_text`：native
block_on fs；wasm 同步 XHR 退化全量。`io_read_chunk(path, off, n)`：
**native 帧级区间口**（分块流式的底层协议——每帧一块交出控制权）；
Web 端 Err（fetch 全量承载）。`io_size`：native fs metadata；wasm 返
None（分块判定退化信号）。加密资产（SFXE1）：只走 `read` 全量解密面
——`io_read_chunk` 明文读不支持分块解密。

[io 走向矩阵] `image.load`/`font.load`/`mixer.Sound`/`music.load`/
`music.queue`：**全走 starfish_io 门面**（BASE_DIR+加密语义统一，
2026-11-20 audio 统一批次）；`video.open`：路径直传平台解码器
（MF/MediaPlayer/HTML5 video 自治流式——加密 video 挂账）。
```

## <span class="c-func">函数</span>

#### <span class="c-func">read(path) -> bytes</span>

读资源全部字节（`.sfx` 密文自动解密，明文资源原样透传）。
**[差异]** Web 端 = 同步 XHR 退化全量（**裸读**——不走加密面，加密资产
Web 端走 `io.fetch` 异步面）。

- :param path: 资源路径
- :type path: str
- :returns: 资源字节
- :rtype: bytes

#### <span class="c-func">read_text(path) -> str</span>

读文本（UTF-8，非法序列替换）。

- :param path: 资源路径
- :type path: str
- :rtype: str

#### <span class="c-func">write(path, data)</span>

覆盖写（str 按 UTF-8 编码）。**[差异]** Web 端不可用。

- :param path: 目标路径
- :type path: str
- :param data: 写入内容
- :type data: bytes | str

#### <span class="c-func">write_text(path, text)</span>

覆盖写文本（UTF-8）。**[差异]** Web 端不可用。

- :param path: 目标路径
- :type path: str
- :param text: 文本内容
- :type text: str

#### <span class="c-func">exists(path) -> bool</span>

路径存在性。**[差异]** Web 端不可用。

- :param path: 资源路径
- :type path: str
- :rtype: bool

## <span class="c-func">函数</span>（续）——异步统一口

#### <span class="c-func">fetch(path) -> bytes</span> <span class="tag async">async</span>

底层通用异步读取口——取回资源字节（`.sfx` 密文自动解密）。

```python
async def main():
    data = await pygame.io.fetch("assets/map.json")
```

- :param path: 资源路径
- :type path: str
- :rtype: bytes

#### <span class="c-func">payload_list() -> list</span>

打包清单——runtime 单元内的资源路径表（native = 目录扫描；wasm =
打包清单）。

- :rtype: list

## <span class="c-func">异步桥口</span>

两拍 IO 桥的取数 / 判据 / 压队口（三平台可用；普通脚本走 `image.load`
等三线接口即可，无需直触）。

#### <span class="c-func">io_ready(path) -> bool</span>

该路径的异步结果是否已入完成表（恢复判据）。

- :param path: 资源路径
- :type path: str
- :rtype: bool

#### <span class="c-func">io_take(path) -> bytes</span>

按路径取走异步 IO 结果（恢复前必已就绪，调用方须先过 `io_ready`；
多任务并发 io 按路径隔离不串数据）。

- :param path: 资源路径
- :type path: str
- :rtype: bytes

#### <span class="c-func">io_pending_append(path)</span>

任务级 io 挂起压队（tick 收 io_read 挂起时调用——每帧排空并发起 fetch）。

- :param path: 资源路径
- :type path: str
