# pygame.tasks

协程任务原语——**协作式微调度器**（帧泵驱动：Rust 帧边界统一推进任务表，
单线程零竞态）。配合 `async def main()` 入口使用——网络等待、多域拆分、
生命周期管理共用一套原语。

```{note}
📌 **挂起点数量定约**：任务 while 体内的 `yield_now()` 必须**恰好 1 个**
——0 个 = 任务一路跑至结束（静默死亡）；2 个 = 每 tick 两轮体
（物理/逻辑 2 倍速）。**嵌套协程内写外层局部变量必须 `nonlocal` 全列**
——漏声明 = UnboundLocalError 静默死任务（调度器异常存档不炸帧循环）。
```

## 快速上手

```python
import pygame

running = [True]                       # 全局退出旗标（list 原位改，跨协程共享）

async def logic_task(world):
    clock = pygame.time.Clock()
    while running[0]:
        dt = clock.get_time() / 1000.0 # 真实帧间隔（accumulator 喂食源）
        world.step_frame(dt)
        await pygame.tasks.yield_now() # 让出——帧边界恢复

async def render_task(screen):
    while running[0]:
        with screen.render() as t:
            ...
        pygame.display.flip()
        await pygame.tasks.yield_now()

async def main():
    screen = pygame.display.set_mode((800, 600))
    pygame.tasks.spawn(logic_task(world))
    pygame.tasks.spawn(render_task(screen))
    while running[0]:
        for e in pygame.event.get():
            if e.type == pygame.event.QUIT:
                running[0] = False
                return
        await pygame.tasks.yield_now() # 保活泵（任务帧泵自动推进）
```

## <span class="c-func">函数</span>

#### <span class="c-func">spawn(coro, name=None) -> Task</span>

把协程登记进任务表——**帧泵每帧自动推进**（tick 一次 = 协程跑到下一
挂起点）。返回 Task 对象（取证口：`failed()` / `exception()` /
`done` / `result`）。

- 传入已有 Task 则幂等返回（gather 复用形态）
- name 用于任务失败定位（`t.exception()` 时可读）

#### <span class="c-func">gather(*coros) -> awaitable</span>

并发推进多个协程——**全部完成才解除 await**；首个失败任务重抛其
异常（asyncio gather 同款语义，失败定位 = 任务 name）。

```python
results = await pygame.tasks.gather(net_task(), sim_task())
```

#### <span class="c-func">yield_now() -> awaitable</span>

**协作式让出原语**——交出控制权到帧边界，下一帧恢复执行。规范名
（旧 `pygame.next_frame` 已退役）。

```python
async def net_task(sock, queue):
    while running[0]:
        data = sock.recv(4096)         # 同步 socket 本体（非阻塞）
        if data is None:
            await pygame.tasks.yield_now()   # 无数据——让出，帧循环继续
            continue
        queue.put(data)
```

#### <span class="c-func">sleep_frames(frames) -> awaitable</span>

挂起指定帧数（帧泵计数恢复）。

#### <span class="c-func">sleep(seconds) -> awaitable</span>

挂起指定秒数（内部换算帧数——固定步长 1/60）。

#### <span class="c-func">wait_for(coro, seconds) -> awaitable</span>

spawn + 截止帧——超时自动 `cancel` 任务并抛 `TimeoutError`
（防"以为有界实际无限"的任务泄漏）。

#### <span class="c-func">frame() -> int</span>

当前帧泵计数（调度器已推进的帧数——取证/节奏参考）。

#### <span class="c-class">Queue</span>

协程间消息队列——**awaitable get**（协作式轮询，单线程无竞态）。

```python
queue = pygame.tasks.Queue()

async def consumer():
    while running[0]:
        data = await queue.get()       # 空队挂起，入队帧恢复
        handle(data)

queue.put(payload)                     # 即时入队（consumer 下一 tick 取）
```

- `put(v)`：即时入队
- `get()`：awaitable（空队挂起）
- `get_nowait()` / `q_pop()`：非阻塞取（空队返回 None）

## 退役说明

- `pygame.next_frame`：已退役（2026-11-22 定名批次）——规范名 =
  `pygame.tasks.yield_now()`，顶层挂载已移除。
- socket 的**同步本体**（`pygame.net.TCP.recv` 等）保持 pygame 传统
  语义——协程形态用 `yield_now()` 轮询封装（库级 `arecv` 封装待加）。
