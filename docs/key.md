# pygame.key

键盘输入面（K_* 常量全表 / 按下视图 / 修饰键位集）。

> 🚧 **页面建设中**——K_* 常量全表见 `pygame.key` 模块（绑定源码
> `src/rpy/pygame/key.rs` 为事实源）。

## 函数

### <span class="bd bd-ok">函数</span> `get_pressed() -> Pressed`

按下视图（`Pressed.get(key) -> bool` 非阻塞查询）。

### <span class="bd bd-magic">魔术方法</span> 相关

`Key.__eq__` 值语义比对见 [pygame.event](event.md#Key)。
