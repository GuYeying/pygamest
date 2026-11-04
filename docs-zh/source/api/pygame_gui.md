# pygame_gui

egui 即时模式 GUI——**独立顶层模块**（`import pygame_gui`；非
`pygame.*` 子模块）。接口定约为"第三方库等价物，不 pygame 化"——
控件方法名 = egui 原生词汇。

```python
import pygame_gui

class HUD(pygame_gui.UiBase):
    def __init__(self):
        self.hp = 100

    def draw(self, ui):
        ui.heading("状态")
        if ui.button("回血"):
            self.hp = 100
        self.hp = ui.slider_f32("HP", self.hp, 0, 100)

def main():
    pygame.display.set_mode((800, 600))
    pygame_gui.init()
    hud = HUD()
    while True:
        for e in pygame.event.get():
            pygame_gui.event(e)          # 显式喂事件——不喂则无输入
            if e.type == pygame.event.QUIT:
                return
        with screen.render() as t:
            t.draw_ui(hud)               # 恒在游戏内容之上
        ...
```

## <span class="c-func">函数</span>

#### <span class="c-func">init()</span>

建 egui Context / 输入桥 / 绘制端。**须在 `pygame.display.set_mode`
之后调用**（幂等；未满足报 RuntimeError）。

**[差异]** init **不自动捕获事件**——输入需 `event(e)` 逐条显式喂
（捕获行为在数据流层完全可见）。CJK 字体自动注入（Windows msyh /
Web cjk.ttf / Android 系统候选链，miss 回退系统探测——仅桌面）。

#### <span class="c-func">event(e)</span>

逐条喂事件——在 `pygame.event.get()` 循环内调用。pygame 事件 → 引擎
事件**全保真重建**（`e.mod` KMOD 位 → 完整修饰键，egui 快捷键可用；
鼠标键 1..5 → 左/中/右/后退/前进）；egui 无关变体（quit / touch /
user…）静默 no-op。

#### <span class="c-func">wants_pointer() -> bool</span>

egui 是否想要指针输入（游戏让位判断）。未初始化 = False。

- :rtype: bool

#### <span class="c-func">wants_keyboard() -> bool</span>

egui 是否想要键盘输入。未初始化 = False。

- :rtype: bool

#### <span class="c-func">shutdown()</span>

卸载运行时（幂等；未执行的延迟排程由引擎丢弃并安全清理）。

## <span class="c-class">类</span>

### <span class="c-class">UiBase —— 面板基类</span>

面板 = `UiBase` 子类实例（isinstance 强制校验，鸭子类型不放行）——
必须实现 `draw(self, ui)`；面板状态与游戏对象经**实例属性**捕获。

##### <span class="c-func">draw(ui)</span>

每帧 UI 构建回调（框架注入 `ui` 调用）；基类实现直接抛
`NotImplementedError`。

### <span class="c-class">Ui —— 控件宿主（框架注入）</span>

`panel.draw(ui)` 的回调参数——**不可手工构造**（不挂模块命名空间）。
**仅在 draw 回调栈内有效**，勿跨帧持有（回调外任何方法报 TypeError）。

##### <span class="c-func">heading(text) / label(text)</span>

一级标题 / 文本标签。

##### <span class="c-func">button(text) -> bool</span>

按钮——本帧被点击 = True。

- :rtype: bool

##### <span class="c-func">separator() / add_space(px)</span>

分隔线 / 留白（像素）。

##### <span class="c-func">checkbox(text, checked) -> bool</span>

复选框——返回新状态。

- :rtype: bool

##### <span class="c-func">slider_f32(text, value, min, max) -> float</span>

滑条——返回新值。

- :rtype: float

##### <span class="c-func">text_edit(text) -> str</span>

单行文本框——返回本帧内容。**[差异]** 每帧以传入值重置——值状态由
脚本侧保存。

- :rtype: str

##### <span class="c-func">radio_group(options, selected) -> int</span>

单选组——返回本帧选中序号。

- :rtype: int

##### <span class="c-func">selectable(text, selected) -> bool</span>

可选行——本帧被点击 = True。

- :rtype: bool

##### <span class="c-func">progress_bar(fraction)</span>

进度条（`0.0`..`1.0` 自动钳制，带百分比文本）。

##### <span class="c-func">drag_f32(text, value, speed=None, min=None, max=None) -> float</span>

数值拖拽（text 作前缀标签；`min` / `max` 同时提供才启用范围钳制）。

- :rtype: float

##### <span class="c-func">combo_box(label, options, selected) -> int</span>

下拉选择——返回本帧选中序号。**[差异]** 同 `label` 的两个下拉共享
内部 id——`label` 须全局取唯一名。

- :rtype: int

##### <span class="c-func">horizontal(callback=None)</span>

水平布局双形态：`ui.horizontal(cb)` 回调形态（egui 原生，cb 收到新
`Ui`）；无参返回 `UiRegion` 区域对象——配 `with ui.horizontal():`
Python 惯用形态。

```python
def draw(self, ui):
    with ui.horizontal():
        ui.label("HP:")
        ui.progress_bar(self.hp / 100)
```

### <span class="c-class">UiRegion —— with 布局区</span>

`ui.horizontal()` 无参形态的返回物（仅供 `with`；不可手工构造）。
`__exit__` 回填父光标（不抑制块内异常）；跨帧持有报 TypeError。

## <span class="c-func">会话集成 —— 两条绘制通路</span>

实现本体在本模块，方法宿主在 render 会话对象上。

#### <span class="c-func">t.draw_ui(panel)</span>（t = `with screen.render()` 会话）

主屏绘制通路——**指针交互主目标**（窗口指针直通）。每帧一次；恒在
游戏内容之上、恒单次落屏（帧末统一落屏——HUD 层级语义由结构保证）。
坐标系 = **逻辑画布**（`set_mode` 语义尺寸，与游戏内容同系）。

- :param panel: 面板（UiBase 子类实例）
- :type panel: UiBase

#### <span class="c-func">t.draw_ui_on_surface(panel)</span>（t = `with surf.render()` 会话）

离屏绘制通路——**纯展示形态**（画中画 / 游戏内面板）：剥除窗口指针
事件，面板显示完全由脚本侧变量驱动（变量驱动定约——交互在游戏侧
判定，传值不回写）。像素即逻辑点（无 DPI 概念）。

- :param panel: 面板（UiBase 子类实例）
- :type panel: UiBase
