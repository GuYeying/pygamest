# Android

## 获取

`pygamestdev_demo.apk`（约 25MB）——全部案例脚本 + 模型资产内置，装上即
启动器逐案例点选运行（未启用能力轴的案例 import 即 `ModuleNotFoundError`，
属预期裁剪语义）。

```bash
adb install -r pygamestdev_demo.apk
adb shell am start -n pygame.rsdev/pygamestdev.MainActivity
```

## 网络案例（net_test）前置

```bash
adb reverse tcp:9001 tcp:9001          # USB 通道映射宿主端口
python net_echo_server.py              # 宿主起回声服（0.0.0.0:9001）
```

同网段真机不走 USB：案例内候选含 PC 局域网 IP 直连。

## 资产覆盖

案例自带资产随 APK 内置；单案例调试可覆盖 filesDir：

```bash
adb push cloth.png /data/local/tmp/
adb shell "run-as pygame.rsdev sh -c 'cp /data/local/tmp/cloth.png files/cloth.png'"
```

## 输入

- 触摸/多点触控：事件流直通（见 `touch_test.py`）
- 文本输入：`TextInputSurfaceView` 桥（IME 全链——点输入框弹键盘、
  退格、失焦收起，案例 `ime_egui_demo.py`）
- 手柄：`gamepad_test.py`
