# pygame.io

资源与存档读写——桌面同步直读直写；Web v1 不提供同步面（资源装载由
宿主预加载 / `image.load` 等三线接口承载）。

```{note}
[基准目录] Python 面恒以 **main 所在目录**为基准写相对路径——打包形态
不改写调用写法（Android 基准目录由引擎 run 自动注入，`set_base_dir`
不暴露）。
```

## <span class="c-func">函数</span>

#### <span class="c-func">read(path) -> bytes</span>

读资源全部字节（`.sfx` 密文自动解密，明文资源原样透传）。
**[差异]** Web 端调用报错——v1 不提供同步 io。

- :returns: 资源字节
- :rtype: bytes

#### <span class="c-func">read_text(path) -> str</span>

读文本（UTF-8，非法序列替换）。

- :rtype: str

#### <span class="c-func">write(path, data)</span>

覆盖写（`bytes` 或 `str`——str 按 UTF-8 编码）。**[差异]** Web 端不可用。

#### <span class="c-func">write_text(path, text)</span>

覆盖写文本（UTF-8）。**[差异]** Web 端不可用。

#### <span class="c-func">exists(path) -> bool</span>

路径存在性。**[差异]** Web 端不可用。

- :rtype: bool

## <span class="c-func">函数</span>（续）——异步统一口

#### <span class="c-func">fetch(path) -> bytes</span> <span class="tag async">async</span>

底层通用异步读取口——取回资源字节（`.sfx` 密文自动解密）。

```python
async def main():
    data = await pygame.io.fetch("assets/map.json")
```

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

- :rtype: bool

#### <span class="c-func">io_take(path) -> bytes</span>

按路径取走异步 IO 结果（恢复前必已就绪，调用方须先过 `io_ready`；
多任务并发 io 按路径隔离不串数据）。

- :rtype: bytes

#### <span class="c-func">io_pending_append(path)</span>

任务级 io 挂起压队（tick 收 io_read 挂起时调用——每帧排空并发起 fetch）。
