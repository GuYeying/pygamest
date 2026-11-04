# pygame.net

联网——TCP 消息分帧连接 + UDP 数据报套接字。**轮询式语义**：帧内
`send` / `recv`，主线程永不阻塞。

```{note}
[TCP 分帧] 4 字节大端长度前缀消息分帧，单帧上限 16 MB；原生后台线程
+ 阻塞流，**Web 自动 WebSocket**（addr 写 `ws://host:port`）——三平台
同一轮询面。
```

## <span class="c-class">类</span>

### <span class="c-class">TCP —— 消息分帧连接</span>

#### <span class="c-func">TCP(addr)</span>

发起连接（立即返回，后台握手；`Connecting` 期间 `send` 入队缓冲）。

```python
conn = pygame.net.TCP("127.0.0.1:9000")
# Web: pygame.net.TCP("ws://127.0.0.1:9000")
```

- :param addr: `"host:port"`（Web = `"ws://host:port"`）

##### <span class="c-func">send(data)</span>

发一帧（`bytes` 或 `str`——str 按 UTF-8 编码）。

##### <span class="c-func">recv() -> bytes | None</span>

收一帧（**非阻塞**；无帧 = None，下帧再查）。

- :rtype: bytes | None

##### <span class="c-func">state() -> str</span>

连接态：`"connecting"` / `"connected"` / `"closed"`。

- :rtype: str

##### <span class="c-func">close()</span>

关闭连接。

### <span class="c-class">UDP —— 数据报套接字</span>

非阻塞数据报。**[差异]** 原生平台专属——Web 端构造 / 调用即报错
（Web 不支持 UDP）。

#### <span class="c-func">UDP(local_addr)</span>

绑定本地端口。

- :param local_addr: `"host:port"`

##### <span class="c-func">send_to(data, peer)</span>

发一报（`peer` = `"host:port"`）。

##### <span class="c-func">recv_from() -> tuple[bytes, str] | None</span>

收一报（非阻塞）——`(数据, 对端地址)`。

- :rtype: tuple[bytes, str] | None

##### <span class="c-func">set_broadcast(on)</span>

广播开关。
