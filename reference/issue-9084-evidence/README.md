# Issue #9084 实测数据包

> 为 [RustPython/RustPython#9084](https://github.com/RustPython/RustPython/issues/9084)
> （android bionic "out of TLS keys"）补充**可独立复现的实测数据**。
> 背景：官方至今无回复，疑似被当成 AI 生成内容搁置。本包把每一层论断都变成
> "维护者可一条命令重跑"的实验。

## 包内容

| 文件/目录 | 是什么 | 对应实验 |
|---|---|---|
| `census.sh` | vm+common 内 thread_local!/static_cell! 静态的括号感知普查脚本 | E2 |
| `ir_probe/` | 依赖真实 vendored rustpython-vm（同 feature 集）的探针 crate | E3 |
| `keys_probe/` | 真机 pthread key 池探测 bin（1024 个 RP 同形状静态 + key 池普查） | E4 |
| `issue-comment-draft.md` | 英文评论稿（贴到 issue 用），`⟨E3/E4 占位⟩` 处填实测数字 | — |

## E1 — std 在 android 上把每个 thread_local! 都走 pthread key（已测毕）

三个微型探针（const 无析构 / const 带析构 / 惰性初始化），rustc 1.97.1：

```bash
rustc --target aarch64-linux-android --crate-type=lib --emit=llvm-ir probe.rs
grep -c "LazyKey\|pthread_setspecific" probe.rs.ll   # android: 全部命中
grep -c "llvm.threadlocal" probe.rs.ll               # android: 0（无原生 TLS）
```

| 静态形态 | aarch64-linux-android | x86_64-unknown-linux-gnu |
|---|---|---|
| const-init 无析构 | **LazyKey + pthread_setspecific** | llvm.threadlocal.address（零 OS key） |
| const-init 带析构 | **LazyKey + pthread_setspecific** | __cxa_thread_atexit_impl（零 key） |
| 惰性初始化 | **LazyKey + pthread_setspecific** | llvm.threadlocal.address |

结论：**bionic 上每个 thread_local! 静态 = 1 个进程级 pthread key**（首次访问时经
`LazyKey::lazy_init` 申请）；桌面同类静态零 key。这就是"CI 绿灯、真机必崩"的根源。

## E2 — 源码普查（已测毕：37 个）

```bash
./census.sh   # 缺省路径指向本仓 vendored 快照；也可传任一 RustPython checkout 路径
```

实测（23fcb8d9）：vm 32 + common 5 = **37 个 thread_local! 静态**
（大头：`vm/thread.rs` 7 个、`common/refcount.rs` 2 个、`stdlib/sys/monitoring.rs` 2 个）。

## E3 — 真实 vm 编译产物级统计（已测毕）

**同代码同 lockfile 的两组构建对照（`--emit=llvm-ir`）：**

| 指标 | aarch64-linux-android（230 个 crate IR） | x86_64-pc-windows-msvc（145 个 crate IR） |
|---|---|---|
| `llvm.threadlocal` 内建引用 | **0** | 3174 |
| `LazyKey` 引用 | **243** | 0 |
| `pthread_setspecific` 调用点 | **132**（分布 18 个 crate） | 0 |

35 个源码静态名中 **22 个**在 android IR 里以命名 TLS 全局在场（`INT_FREELIST023___RUST_STD_INTERNAL_VAL`
形态：8B key 槽 + `destroy_value` 析构指针）；14 个缺席与 cfg 预测完全吻合
（threading 门控 7+2、windows-only signal、死代码消除）。

即：genesis 前解释器要经 LazyKey 申请 ~22 个进程级 pthread key（freelist 群在类型
创建时触碰、codecs 在 importlib 引导时触碰），而同一份代码在桌面目标零 key。

完整数字与 IR 样本见 `ir-results.txt`；复现命令：

```bash
cd ir_probe
export CC_aarch64_linux_android="D:/SDK/Android/ndk/30.0.16248370/toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android26-clang.cmd"
export AR_aarch64_linux_android="D:/SDK/Android/ndk/30.0.16248370/toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-ar.exe"
RUSTFLAGS="--emit=llvm-ir -C debuginfo=0" cargo build --release --target aarch64-linux-android
RUSTFLAGS="--emit=llvm-ir -C debuginfo=0" cargo build --release --target x86_64-pc-windows-msvc
# 统计命令见 issue-comment-draft.md §2
```

> 注意：ir_probe 必须用 pygamest 的 Cargo.lock（已内置拷贝）——全新 lock 会解析出
> 快照之后的新版传递依赖，vendored codegen 编译不过。

```bash
cd ir_probe
export CC_aarch64_linux_android="D:/SDK/Android/ndk/30.0.16248370/toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android26-clang.cmd"
export AR_aarch64_linux_android="D:/SDK/Android/ndk/30.0.16248370/toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-ar.exe"
RUSTFLAGS="--emit=llvm-ir -C debuginfo=0" cargo build --release --target aarch64-linux-android

# 统计：LazyKey 实例化数 / pthread_setspecific 调用点 / 原生 TLS 全局数
grep -c "LazyKey"             target/aarch64-linux-android/release/deps/*.ll | awk -F: '{s+=$2} END{print "LazyKey refs:", s}'
grep -cE "call.*pthread_setspecific" target/aarch64-linux-android/release/deps/*.ll | awk -F: '{s+=$2} END{print "setspecific calls:", s}'
grep -c "llvm.threadlocal"    target/aarch64-linux-android/release/deps/*.ll | awk -F: '{s+=$2} END{print "native TLS globals:", s}'
```

对照组：同 crate `--target x86_64-unknown-linux-gnu` 再跑一遍（预期 LazyKey=0、
llvm.threadlocal 大量）——同代码在 linux 上零 key。

## E4 — 真机 key 池探测（需要真机跑，产出最终一锤定音的两个数）

```bash
cd keys_probe
cargo ndk --platform 26 -t arm64-v8a -o ./out build --release
adb push out/arm64-v8a/keys_probe /data/local/tmp/
adb shell chmod +x /data/local/tmp/keys_probe
adb shell /data/local/tmp/keys_probe          # 阶段1：启动时剩余 key 池 F
adb shell /data/local/tmp/keys_probe 32       # 阶段2：摸 N 个静态，二分找崩溃阈值 T
```

> 设备走卓易通/APK 形态时：把 probe 逻辑塞进现有 testhost APK 的 example
>（参照 `sig_probe_android` 模式），logcat 里抓 `[keys_probe]` 与
> `fatal runtime error: out of TLS keys`。

**要填的两个数：**
- 启动时剩余 key 池 `F` = ___（预期远小于 1024 → 宿主兼容层已大量占用）
- 崩溃阈值 `T` ≈ F（摸到第 T 个静态时 `out of TLS keys`）→ RP 的 37 个静态足以引爆

## 发布建议

- 用自己 GitHub 账号贴 `issue-comment-draft.md`，第一人称、贴关键数字即可；
  **主动修正自己原报告里"数百个"的量级**（实测 37）是可信度最强的信号——AI 内容不会自我纠错。
- 命令块保持原样可复制，别加表情/营销语气。
- 同步在 RustPython Discord（README 有链接）丢一条 issue 链接。
