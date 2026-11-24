# pygame.tasks 实现（**归位定约 2026-11-24**：异步模块源码在 pygame
# 模块树下——host.rs 帧桥 include_str! 拼接执行；py 命名空间挂载
# pygame.tasks 不变。）
#
# ── 帧泵驱动的协作式微调度器 ──
# 任务 = python 协程；帧泵（Rust 侧）每帧调 _async_pump() 推进任务表；
# 单线程零竞态；任务异常存档不炸帧循环（asyncio 同款语义）。
#
# ── 外部名字契约（与 host.rs 双向锁定——改名即断）──
#   host.rs 引用：  _async_pump（帧泵入口）/ _async_ns（挂载 pygame.tasks）
#   host.rs 注入：  _io_ready / _io_pending_append（io 桥 late-bind）
#   host.rs 定义：  _next_frame_state（yield_now 的 awaitable，第 33 行）
#   公开面：        spawn/gather/sleep_frames/sleep/wait_for/Queue/frame/yield_now
#
# ── RP 协程三坑实录（改动前必读——本文件所有"怪写"皆源于此）──
#   ① 双 send：RP 的 await gen 委派是两拍语义——第一 send 恢复到 gen 的
#      yield 挂起，第二 send 才完成 await 表达式。单 send = 任务帧率减半
#      （2026-11-22 泵帧取证实证：pump_frame=120 时 frame=60）。
#   ② 裸 generator can't be awaited：await 只认 __await__ 方法对象/
#      coroutine——挂起原语一律实例形态 + __await__（_TaskWait 先例）。
#   ③ 零 yield 病态：立即 StopIteration 的 gen send 有病态路径——
#      等待型 awaitable 恒先 yield 一帧规避（2026-11-19 send 嵌套
#      重入实测——挂起点推进交帧泵，awaitable 内不 send）。


# ═══ ① 任务体 ═══

class _AsyncTask:
    """单任务：协程持有 + 唤醒帧/io 门 + tick 推进 + 异常存档。"""

    def __init__(self, coro, name=None):
        self.coro = coro
        self.name = name or "task"
        self.done = False
        self.cancelled = False
        self.result = None
        self.exc = None   # 异常存档。属性名不能叫 exception——会遮蔽同名方法
        self.wake_frame = 0          # sleep_frames 截止帧（0 = 立即可跑）
        self.wait_io = False         # io 门：未就绪不 send（2026-11-03 修）
        self.io_path = None

    # ── 帧推进（_async_pump 逐任务调用）──

    def tick(self, now_frame):
        if self.done or now_frame < self.wake_frame:
            return
        if self.wait_io:
            # 提前恢复 = 病态挂起/空数据——按路径完成表判定就绪才放行
            if not _io_ready(self.io_path):
                return
            self.wait_io = False
            self.io_path = None
        try:
            req = self.coro.send(None)
            self._dispatch(req, now_frame)
        except StopIteration as e:
            self._finish(e.value)
        except BaseException as e:
            self._fail(e)

    def _dispatch(self, req, now_frame):
        """挂起请求分派（协程 yield 出的控制值）。

        返回 True = 本 tick 终结；False = 请求已登记，下 tick 再看。
        """
        # 双 send 坑①：仅 "frame" 请求需要第二 send 完成 await 表达式
        if type(req) is str and req == "frame":
            try:
                self.coro.send(None)
            except StopIteration as e:
                self._finish(e.value)
                return True
            except BaseException as e:
                self._fail(e)
                return True
            return False
        if type(req) is tuple and req[0] == "io_read":
            # 等 io：压队（Rust 帧首排空发起 fetch）→ 就绪帧恢复取数
            _io_pending_append(req[1])
            self.io_path = req[1]
            self.wait_io = True
        elif type(req) is tuple and req[0] == "sleep_frames":
            self.wake_frame = now_frame + req[1]
        return False

    # ── 状态收束 ──

    def _finish(self, value):
        self.done = True
        self.result = value

    def _fail(self, e):
        # 任务异常捕获存档——一个任务失败不炸帧循环
        self.done = True
        self.exc = e

    def cancel(self):
        # 协作式取消：标记终止 + 协程对象弃置（GC 回收——RP 的 coro.close
        # 委派链有病态风险，v1 不做强制弹出）
        if not self.done:
            self.done = True
            self.cancelled = True
            self.result = None

    # ── 结果读取 ──

    def failed(self):
        return self.exc is not None

    def exception(self):
        return self.exc

    def wait(self):
        # awaitable 主口（坑②③：实例形态 + 先 yield 一帧守卫）
        return _TaskWait(self)


# ═══ ② 帧泵 ═══

_async_tasks = []
_async_frame = 0


def _async_pump():
    """帧泵每帧调用：帧号 +1 并逐任务推进（Rust 帧边界统一入口）。"""
    global _async_frame
    _async_frame += 1
    for t in _async_tasks:
        t.tick(_async_frame)


def _async_frame_now():
    return _async_frame


# ═══ ③ 挂起原语（awaitable 家族——统一"yield 'frame' 交帧泵"形态）═══

class _SleepFrames:
    """sleep_frames(n)：挂起 n 帧。"""

    def __init__(self, frames):
        self.frames = frames

    def __await__(self):
        # 单次挂起即返回。无限 yield = wake_frame 永远前移 = 死锁（首版实测）
        yield ("sleep_frames", self.frames)


class _Gather:
    """gather(*tasks)：全任务完成才恢复；首个失败任务异常重抛。"""

    def __init__(self, tasks):
        self.tasks = tasks

    def __await__(self):
        while any(not t.done for t in self.tasks):
            yield "frame"
        for t in self.tasks:
            if t.exc is not None:
                raise t.exc   # 失败定位 = t.name / t.exception()
        return [t.result for t in self.tasks]


class _WaitFor:
    """wait_for(coro, frames)：spawn + 截止帧——超时 cancel + TimeoutError
    （防"以为有界实际无限"的任务泄漏）。"""

    def __init__(self, coro, frames):
        self.task = _async_spawn(coro)
        self.deadline = _async_frame + max(1, frames)

    def __await__(self):
        while not self.task.done:
            if _async_frame >= self.deadline:
                self.task.cancel()
                raise TimeoutError("tasks.wait_for 超时")
            yield "frame"
        if self.task.exc is not None:
            raise self.task.exc   # 失败定位 = task.name / exception()
        if self.task.cancelled:
            return None
        return self.task.result


class _TaskWait:
    """Task.wait() 的等待 awaitable（先 yield 一帧守卫——坑③）。"""

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
    """Queue.get() 的等待 awaitable（协作式每帧轮询——单线程无竞态）。"""

    def __init__(self, q):
        self.q = q

    def __await__(self):
        while not self.q.items:
            yield "frame"
        return self.q.items.pop(0)


class _Queue:
    """协作式队列：put 即时入队，get 挂起等取（getter 下一 tick 可见）。"""

    def __init__(self):
        self.items = []

    def put(self, v):
        self.items.append(v)

    def get(self):
        return _QueueGet(self)

    def get_nowait(self):
        return self.q_pop()

    def q_pop(self):
        return self.items.pop(0) if self.items else None


# ═══ ④ 公开 API（pygame.tasks 面）═══

def _async_spawn(coro, name=None):
    # 幂等：传入的已是任务（spawn 产物/gather 复用）直接返回
    if isinstance(coro, _AsyncTask):
        return coro
    t = _AsyncTask(coro, name or "task")
    _async_tasks.append(t)
    return t


def _async_gather(*coros):
    return _Gather([_async_spawn(c) for c in coros])   # 实例即 awaitable


def _seconds_to_frames(seconds):
    # 引擎固定步长 1/60——帧数 = 秒 × 60 向上取整（0.999 补偿浮点）
    return max(1, int(seconds * 60.0 + 0.999))


def _async_sleep_frames(frames):
    return _SleepFrames(max(1, int(frames)))


def _async_sleep(seconds):
    return _SleepFrames(_seconds_to_frames(seconds))


def _async_wait_for(coro, seconds):
    return _WaitFor(coro, _seconds_to_frames(seconds))


def _async_yield_now():
    # 让出原语规范名（定约 2026-11-21：next_frame → tasks.yield_now——
    # "frame" 词根易被误读为帧率控制；yield_now = 协程让出到帧边界）
    return _next_frame_state


# ═══ ⑤ 命名空间装配（host.rs get_item("_async_ns") → pygame.tasks）═══

class _AsyncNs:
    pass


_async_ns = _AsyncNs()
_async_ns.spawn = _async_spawn
_async_ns.gather = _async_gather
_async_ns.sleep_frames = _async_sleep_frames
_async_ns.sleep = _async_sleep
_async_ns.wait_for = _async_wait_for
_async_ns.Queue = _Queue
_async_ns.frame = _async_frame_now
_async_ns.yield_now = _async_yield_now
