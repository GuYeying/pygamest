# pygame.io

资源与存档读写模块——统一处理资源加载与数据持久化，三平台语义一致。

路径规则：相对路径基于**脚本所在目录**解析（各打包形态自动注入基准
目录），绝对路径原样使用——同一份脚本跨平台无需修改。

支持加密资源：`.sfx` 后缀的密文文件在读取时自动解密（明文资源原样
透传）。

## 同步读取

### read(path) -> bytes

读取资源的全部字节。`.sfx` 密文自动解密。

注意：**Web 平台上走同步 XHR**（阻塞主线程直到完成），且为裸读——
加密资源在 Web 端请使用异步口 `pygame.io.fetch()`（自动解密）。

```python
data = pygame.io.read("assets/config.json")
```

### read_text(path) -> str

读取文本文件（UTF-8，非法序列自动替换）。

```python
text = pygame.io.read_text("assets/story.txt")
```

### write(path, data)

覆盖写入。`data` 为 bytes 时原样写入，为 str 时按 UTF-8 编码。

注意：**Web 平台上走同步 XHR POST**（2026-11-23 修复——曾直接报错；
Rust 层 fetch POST 早已实现，绑定层补齐"同步口退化承载"定约）。
POST 到目标地址的保存端点，`Content-Type: application/octet-stream`，
服务端负责落盘——部署服务端须提供 POST 保存能力（与
`pygame.image.save` 的 Web 保存链同端点契约）；Web 写入为裸写
（无 `.sfx` 加密封装语义）。

```python
pygame.io.write("saves/save1.json", data)
```

### write_text(path, text)

覆盖写入文本（UTF-8 编码）。Web 平台同 `write()`（同步 XHR POST）。

### exists(path) -> bool

判断路径是否存在。注意：Web 平台上为同步 XHR 探测。

## 异步读取

### fetch(path) -> bytes

底层通用异步读取口——**推荐的大文件加载方式**。读取过程不阻塞游戏
循环：桌面在后台线程分块读取（每帧交出控制权），Web 端为浏览器异步
全量加载。`.sfx` 密文自动解密。

```python
async def main():
    data = await pygame.io.fetch("assets/map.json")
```

`pygame.image.load()` / `pygame.font.load()` / `pygame.mixer.load()`
等资源加载接口内部均走同一 IO 门面（路径解析与加密语义统一）。

## 辅助口

以下三个口为两拍 IO 桥的取数 / 判据 / 压队机制——普通脚本使用
`pygame.image.load()` 等三线接口即可，无需直接接触。

### io_ready(path) -> bool

该路径的异步结果是否已就绪。

### io_take(path) -> bytes

取走该路径的异步读取结果（须先通过 `io_ready()` 确认就绪；多任务
并发加载按路径隔离，不串数据）。

### io_pending_append(path)

将该路径压入异步加载队列（每帧排空并发起读取）。

## 读取走向一览

| 接口 | 桌面 / 移动 | Web |
|---|---|---|
| read / read_text / exists | 本地文件系统 | 同步 XHR 全量（裸读/裸探测） |
| write / write_text | 本地文件系统（覆盖写） | 同步 XHR POST（保存端点，裸写） |
| io_read_chunk（分块流式） | 帧级区间读取 | 不可用（fetch 全量承载） |
| io_size | 文件元数据 | 返回 None |
| fetch / 资源加载三线口 | 后台线程分块 | 浏览器异步全量 |
| video.open | 平台解码器自治流式 | WebCodecs |
