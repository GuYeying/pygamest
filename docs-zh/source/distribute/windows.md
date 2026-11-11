# Windows 打包

```bash
python scripts/build_windows.py            # pack.toml 单一配置源
python scripts/build.py --target windows   # 总控分派
```

## 产物

```
dist/<game>/
  <game>.exe        ← 净壳 + 入口编译期加密嵌入（磁盘零 main 文件）
  assets/…          ← 哈希名 .sfx 散文件（资源目录镜像源结构）
  _payload.sfx      ← 加密逻辑路径清单（壳启动解密装表）
```

## 五步链

1. `cargo build --release --bin runtime`（build.rs 编译期读 pack.toml
   —— RES_DIR_NAME / 入口加密嵌 exe）
2. 资源 → bundle.py（ChaCha20Poly1305 加密 + 哈希名；视频明文豁免）
3. 脚本收集 → 加密（剔除资源子树；入口不入部署）
4. 清单 `<资源目录>/_payload.sfx`（fetch→register→import 定约）
5. 组装 dist/

## 配置（pack.toml）

```toml
[game]
name = "my_game"
[scripts]
root = "examples/my_game"
[resources]
dir = "examples/my_game/assets"
```

密钥 `assets.key` 每游戏一份（缺失自动生成；**分发勿携**）。
