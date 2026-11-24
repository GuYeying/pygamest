# pygame.tasks 实现（**归位定约 2026-11-24**：异步模块源码在 pygame
# 模块树下——host.rs 帧桥 include_str! 拼接执行；py 命名空间挂载
# pygame.tasks 不变。本文件由 host.rs FRAME_BRIDGE 抽出，逻辑原样。）
# ── 帧泵驱动的协作式微调度器 ──
class _AsyncTask:
    def __init__(self, coro, name=None):
        self.coro = coro
        self.name = name or "task"
        self.done = False
        self.cancelled = False
        self.result = None
        self.exc = None         # 任务异常存档（**exc 属性**——exception() 方法读此；
        # 属性名 exception 会遮蔽同名方法 = t.exception() 调实例炸，2026-11-19）
        self.wake_frame = 0
        self.wait_io = False
        self.io_path = None

    def tick(self, now_frame):
        if self.done or now_frame < self.wake_frame:
            return
        if self.wait_io:
            # io 未就绪不 send（提前恢复 = 病态挂起/空数据——按路径
            # 完成表判定，2026-11-03 修）
            if not _io_ready(self.io_path):
                return
            self.wait_io = False
            self.io_path = None
        try:
            req = self.coro.send(None)
            # **双 send 完成 while 体**（RP 的 await gen 委派两拍语义——
            # 第一 send 恢复到 gen 的 yield 挂起，第二 send 完成 await
            # 表达式让 while 体跑完到下一挂起点；单 send = 任务帧率减半，
            # 2026-11-22 泵帧取证实证 pump_frame=120 时 frame=60）
            if type(req) is str and req == "frame":
                try:
                    self.coro.send(None)
                except StopIteration as e2:
                    self.done = True
                    self.result = e2.value
                    return
                except BaseException as e2:
                    self.done = True
                    self.exc = e2
                    return
            if type(req) is tuple and req[0] == "io_read":
                # 任务挂起等 io：压队（Rust 帧首排空发起 fetch）→
                # 下一 tick 起查就绪，就绪帧恢复取数
                _io_pending_append(req[1])
                self.io_path = req[1]
                self.wait_io = True
            elif type(req) is tuple and req[0] == "sleep_frames":
                self.wake_frame = now_frame + req[1]
        except StopIteration as e:
            self.done = True
            self.result = e.value
        except BaseException as e:
            # 任务异常捕获存档（asyncio 同款语义——一个任务失败不炸帧循环）
            self.done = True
            self.exc = e

    def cancel(self):
        # 协作式取消：标记终止 + 协程对象弃置（GC 回收——RP 的
        # coro.close 委派链有病态风险，v1 不做强制弹出）
        if not self.done:
            self.done = True
            self.cancelled = True
            self.result = None

    def failed(self):
        # 任务异常存档判定（哪个任务失败 = self.name；异常体 = exception()）
        return self.exc is not None

    def exception(self):
        return self.exc

    def wait(self):
        # **awaitable 主口**（返回 _TaskWait 实例——RP 的 await 只认
        # __await__ 方法对象/coroutine，**裸 generator can't be awaited
        # 实测**；_TaskWait.__await__ 恒先 yield 一帧规避立即
        # StopIteration 病态）
        return _TaskWait(self)




_async_tasks = []
_async_frame = 0

def _async_pump():
    global _async_frame
    _async_frame += 1
    for t in _async_tasks:
        t.tick(_async_frame)

class _SleepFrames:
    def __init__(self, frames):
        self.frames = frames
    def __await__(self):
        # 单次挂起（yield 一次即返回——任务下一 tick 恢复执行；
        # 无限 yield 会让 wake_frame 永远前移 = 死锁，首版实测）
        yield ("sleep_frames", self.frames)

class _Gather:
    def __init__(self, tasks):
        self.tasks = tasks
    def __await__(self):
        # 任务推进在 Rust 帧边界统一 pump（同 _FrameAwait——挂起点
        # 同步推进 = send 嵌套重入，2026-11-19 实测）
        while any(not t.done for t in self.tasks):
            yield "frame"
        # 异常 re-raise（asyncio gather 默认语义——首个失败任务重抛，
        # 失败定位 = t.name / t.exception()）
        for t in self.tasks:
            if t.exc is not None:
                raise t.exc
        return [t.result for t in self.tasks]

def _async_spawn(coro, name=None):
    # 幂等：传入的已是任务（spawn 产物/gather 复用）直接返回
    if hasattr(coro, "tick"):
        return coro
    t = _AsyncTask(coro, name or "task")
    _async_tasks.append(t)
    return t

def _async_gather(*coros):
    return _Gather([_async_spawn(c) for c in coros])   # 实例即 awaitable

def _async_sleep_frames(frames):
    return _SleepFrames(max(1, int(frames)))

class _WaitFor:
    # wait_for：spawn + 截止帧——超时 cancel 任务并抛 TimeoutError
    #（asyncio 同名对齐；防"以为有界实际无限"的任务泄漏）
    def __init__(self, coro, frames):
        self.task = _async_spawn(coro)
        self.deadline = _async_frame + max(1, frames)
    def __await__(self):
        while not self.task.done:
            if _async_frame >= self.deadline:
                self.task.cancel()
                raise TimeoutError("tasks.wait_for 超时")
            yield "frame"
        # 任务异常 re-raise（asyncio 对齐——失败定位 = task.name/exc）
        if self.task.exc is not None:
            raise self.task.exc
        if self.task.cancelled:
            return None
        return self.task.result

class _TaskWait:
    # Task 等待 awaitable（实例形态——__await__ 恒先 yield 一帧，
    # 规避 RP 对零 yield/立即 StopIteration gen 的 send 病态）
    def __init__(self, task):
        self.task = task
    def __await__(self):
        yield "frame"
        while not self.task.done:
            yield "frame"
        if self.task.exc is not None:
            raise self.task.exc
        return self.task.result

class _QueueGet:
    def __init__(self, q):
        self.q = q
    def __await__(self):
        # 协作式轮询：getter 每帧查队（无需唤醒机制——单线程无竞态）
        while not self.q.items:
            yield "frame"
        return self.q.items.pop(0)


class _Queue:
    def __init__(self):
        self.items = []
    def put(self, v):
        # 即时入队（协作式：getter 下一 tick 取到）
        self.items.append(v)
    def get(self):
        return _QueueGet(self)   # awaitable
    def get_nowait(self):
        return self.q_pop()
    def q_pop(self):
        return self.items.pop(0) if self.items else None

def _async_sleep(seconds):
    # 秒版 sleep（引擎固定步长 1/60——帧数 = 秒 × 60 向上取整）
    return _SleepFrames(max(1, int(seconds * 60.0 + 0.999)))

def _async_wait_for(coro, seconds):
    return _WaitFor(coro, max(1, int(seconds * 60.0 + 0.999)))

class _AsyncNs:
    pass

_async_ns = _AsyncNs()
_async_ns.spawn = _async_spawn
_async_ns.gather = _async_gather
_async_ns.sleep_frames = _async_sleep_frames
_async_ns.sleep = _async_sleep
_async_ns.wait_for = _async_wait_for
_async_ns.Queue = _Queue
_async_ns.frame = lambda: _async_frame

def _async_yield_now():
    # 协作式让出原语的规范名（定约 2026-11-21：next_frame → tasks.yield_now
    # ——"frame" 词根易被误读为帧率控制；yield_now = 协程让出到帧边界）
    return _next_frame_state
_async_ns.yield_now = _async_yield_now
