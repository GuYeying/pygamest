# Android 打包

```bash
python android/build.py --game <name> --install
```

## 产物

`target/apk/<game>.apk`——Java 宿主 + JNI 嵌入面（NativeActivity 语义：
dlopen libruntime.so 调 android_main，窗口/事件由 starfish-we 经
native_app_glue 自管）。

## 六步链

1. cargo ndk（arm64-v8a，release——RP 恒 release 铁律）
2. llvm-strip + JNI 符号自检
3. 脚本 + 资源 → APK assets（脚本保原名加密 .py + 资源哈希名 .sfx +
   `_manifest.txt` 物理清单——启动经 AssetManager 展开到内部存储）
4. javac + d8（Java 宿主 dex）
5. aapt2 link（orientation 对齐 pack.toml `[android]`）
6. zipalign + apksigner（debug.keystore）

## 配置

`pack.toml [android]`：`package` / `orientation`
（landscape / portrait / sensorLandscape…）。
