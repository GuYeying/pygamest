# pygame.dialog

系统文件对话框——选取文件、另存文件。**[差异]** v1 同步对话框为
**桌面专属**（Windows / Linux / macOS）；Web / Android 调用报错
（异步 job API 后续批次）。

## <span class="c-func">函数</span>

#### <span class="c-func">open_file(title=None, filters=None) -> PickedFile | None</span>

弹出文件选取框。

```python
f = pygame.dialog.open_file(
    "打开关卡",
    [("关卡文件", ["json", "lvl"]), ("全部文件", ["*"])])
if f is not None:
    data = f.read()
```

- :param title: 对话框标题（None = 系统默认）
- :type title: str | None
- :param filters: 过滤项
- :type filters: list[tuple[str, list[str]]] | None
- :returns: 选取的文件；用户取消 = None
- :rtype: PickedFile | None

#### <span class="c-func">save_bytes(file_name, data) -> bool</span>

另存为对话框——把字节存到用户指定位置。

- :param file_name: 建议文件名
- :type file_name: str
- :param data: 内容（str 按 UTF-8 编码）
- :type data: bytes | str
- :returns: True = 已保存 / False = 用户取消
- :rtype: bool

## <span class="c-class">类</span>

### <span class="c-class">PickedFile —— 选中的文件</span>

`open_file()` 产物（由引擎创建，不可手工构造）——桌面持路径惰性读盘。

##### <span class="c-func">name() -> str</span>

文件名（含扩展名）。

##### <span class="c-func">read() -> bytes</span>

读取全部字节。

- :rtype: bytes

## 异步面（2026-11-10 定约）

`open_file` / `save_bytes` 为 **awaitable**——后台线程跑模态对话框，
帧循环不冻（对话框背后画面继续渲染）：

```python
picked = await pygame.dialog.open_file("选个文件", [("文本", ["txt", "md"])])
if picked is not None:          # None = 用户取消
    print(picked.name(), len(picked.read()))

ok = await pygame.dialog.save_bytes("out.txt", b"hello")   # True = 已保存
```

- **机制**：原生 job 三口（`_open_start`/`_save_start`/`_poll`/`_open_take`
  /`_save_take`）+ FRAME_BRIDGE 帧轮询 shim（`src/rpy/jobs.rs` 注册表）
  ——与 `pygame.image.load` 异步化同构（资源/外部应答 = 异步范式）
- **平台**：桌面实装（rfd 后台线程）；Web/Android 后端站桩 ValueError
  （robius 桥后续批次——API 面已就绪，接后端零脚本改动）
