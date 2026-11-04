# pygame.permission

运行时权限申请（Android 主用；桌面 / Web 瞬时通过）。

## <span class="c-const">常量</span>

| 常量 | 语义 |
|---|---|
| MICROPHONE | 麦克风 |
| INTERNET | 网络 |

## <span class="c-func">函数</span>

#### <span class="c-func">ensure(permission) -> bool</span>

同步申请权限，返回是否已授予。

**[差异]** Android `MICROPHONE` 会弹系统授权框**阻塞等待**——自动化
冒烟勿触碰；`INTERNET` 瞬时返回。未知标识抛 `ValueError`。

```python
if pygame.permission.ensure(pygame.permission.MICROPHONE):
    ...
```

- :param permission: 权限标识
- :type permission: int（`MICROPHONE` / `INTERNET`）
- :rtype: bool
