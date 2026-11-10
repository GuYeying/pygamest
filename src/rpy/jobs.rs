//! rpy 原生异步 Job 注册表——**帧轮询式后台任务**（dialog/video 异步面
//! 的机制层）：`spawn` 起线程跑阻塞工作，Python 侧 awaitable 逐帧
//! `yield "frame"` 轮询 `poll`，就绪后 `take` 取结果（构造 PyObject 在
//! 引擎线程——后台线程零 VM 接触）。
//!
//! 与 testhost.rs FRAME_BRIDGE 的关系：那是 Python 微调度器（协程挂起
//! 语义），本模块是它的原生搭档——`("io_read", path)` 双拍模式的"非
//! 字节载荷"推广形态（dialog 结果是对象、video 结果是 Video 句柄）。
//!
//! 平台面：线程承载 = native 专属（wasm 无 thread::spawn——消费方自行
//! cfg 站桩，见 dialog.rs / video.rs 的非桌面分支）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// 单个后台任务（done 位 + 结果槽）
struct Job<T> {
    done: AtomicBool,
    result: Mutex<Option<Result<T, String>>>,
}

/// 进程级注册表（句柄 = 槽位下标；只增不减——进程期对话框/视频打开
/// 次数有界，槽位常驻无泄漏压力）
pub struct Registry<T> {
    slots: Mutex<Vec<Arc<Job<T>>>>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            slots: Mutex::new(Vec::new()),
        }
    }
}

impl<T: Send + 'static> Registry<T> {
    /// 起线程跑 `f`，立即返回句柄（引擎线程零阻塞）
    pub fn spawn(&self, f: impl FnOnce() -> Result<T, String> + Send + 'static) -> i64 {
        let job = Arc::new(Job {
            done: AtomicBool::new(false),
            result: Mutex::new(None),
        });
        let runner = job.clone();
        let _ = std::thread::spawn(move || {
            let r = f();
            *runner.result.lock().unwrap() = Some(r);
            runner.done.store(true, Ordering::Release);
        });
        let mut slots = self.slots.lock().unwrap();
        slots.push(job);
        (slots.len() - 1) as i64
    }

    /// 就绪查询（帧轮询用——恒廉价：一个原子读）
    pub fn poll(&self, handle: i64) -> bool {
        let slots = self.slots.lock().unwrap();
        slots
            .get(handle as usize)
            .map(|j| j.done.load(Ordering::Acquire))
            .unwrap_or(true) // 陈旧句柄视为完成（take 报错兜底）
    }

    /// 取走结果（取走式单次语义；须在 poll=true 后调）
    pub fn take(&self, handle: i64) -> Result<T, String> {
        let slots = self.slots.lock().unwrap();
        let job = slots
            .get(handle as usize)
            .ok_or_else(|| format!("Job 句柄 {handle} 不存在"))?;
        if !job.done.load(Ordering::Acquire) {
            return Err(format!("Job {handle} 未完成（take 须在 poll 就绪后调用）"));
        }
        let mut guard = job.result.lock().unwrap();
        guard
            .take()
            .unwrap_or_else(|| Err(format!("Job {handle} 结果已取走")))
    }
}
