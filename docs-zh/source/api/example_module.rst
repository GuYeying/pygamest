pygame.xxx
==========

.. admonition:: ⚠️ Unstable
    :class: warning

    这个模块尚在初期，API 不稳定，后续可能发生破坏性变更。

    【这里写模块完整描述：作用、定位、整体说明】

.. dropdown:: 模块顶层成员
    :open:
    :animate: fade-in

    .. dropdown:: <span class="tag private">private</span> __internal_const
        :animate: fade-in

        **不暴露的内部常量**

        完整描述：仅供模块内部使用，外部不要引用。

    .. dropdown:: <span class="tag internal">internal</span> _default_config
        :animate: fade-in

        **不推荐外部使用的默认配置**

        完整描述：模块内部默认配置，外部可读取但不建议直接修改。

    .. dropdown:: <span class="tag attr">attr</span> VERSION
        :animate: fade-in

        **对外暴露的版本号常量**

        完整描述：当前模块版本号，格式为 "x.y.z"。

    .. dropdown:: <span class="tag method">method</span> init() -> bool
        :animate: fade-in

        初始化模块，必须在使用其他函数前调用。

        :returns: 初始化成功返回 True，失败返回 False
        :rtype: bool

    .. dropdown:: <span class="tag method">method</span> quit() -> None
        :animate: fade-in

        退出模块，释放资源。

        :returns: 无
        :rtype: None

.. dropdown:: pygame.xxx.Type1
    :animate: fade-in

    .. dropdown:: <span class="tag magic">magic</span> __init__(width: int, height: int) -> Type1
        :animate: fade-in

        构造函数，创建一个 Type1 实例。

        :param width: 宽度，单位像素
        :type width: int
        :param height: 高度，单位像素
        :type height: int
        :returns: 新实例
        :rtype: Type1

    .. dropdown:: <span class="tag private">private</span> __private_attr
        :animate: fade-in

        **不暴露的私有属性**

        完整描述：双下划线开头，Python 名称改写后外部无法直接访问。

    .. dropdown:: <span class="tag internal">internal</span> _internal_attr
        :animate: fade-in

        **不推荐外部使用的内部属性**

        完整描述：单下划线开头，约定为内部使用，外部可读但不建议依赖。

    .. dropdown:: <span class="tag attr">attr</span> width
        :animate: fade-in

        **公开属性：宽度**

        完整描述：只读属性，表示实例的宽度，单位像素。

    .. dropdown:: <span class="tag attr">attr</span> height
        :animate: fade-in

        **公开属性：高度**

        完整描述：只读属性，表示实例的高度，单位像素。

    .. dropdown:: <span class="tag private">private</span> __private_method() -> None
        :animate: fade-in

        不暴露的私有方法。

        完整描述：仅内部使用。

    .. dropdown:: <span class="tag internal">internal</span> _internal_method() -> None
        :animate: fade-in

        不推荐外部调用的内部方法。

        完整描述：内部辅助方法。

    .. dropdown:: <span class="tag method">method</span> resize(new_width: int, new_height: int) -> None
        :animate: fade-in

        调整实例尺寸。

        :param new_width: 新宽度
        :type new_width: int
        :param new_height: 新高度
        :type new_height: int
        :returns: 无
        :rtype: None

    .. dropdown:: <span class="tag method">method</span> draw(surface) -> None
        :animate: fade-in

        在指定 surface 上绘制。

        :param surface: 目标绘图表面
        :type surface: pygame.Surface
        :returns: 无
        :rtype: None

    .. dropdown:: <span class="tag static">static</span> from_string(s: str) -> Type1
        :animate: fade-in

        静态方法：从字符串创建实例。

        调用方式：``pygame.xxx.Type1.from_string("800x600")``

        :param s: 尺寸字符串，格式 "宽x高"
        :type s: str
        :returns: 新实例
        :rtype: Type1

.. dropdown:: pygame.xxx.Type2
    :animate: fade-in

    .. dropdown:: <span class="tag magic">magic</span> __init__(x: float, y: float) -> Type2
        :animate: fade-in

        构造函数，创建一个二维向量。

        :param x: x 坐标
        :type x: float
        :param y: y 坐标
        :type y: float
        :returns: 新实例
        :rtype: Type2

    .. dropdown:: <span class="tag private">private</span> __private_data
        :animate: fade-in

        **不暴露的私有数据**

    .. dropdown:: <span class="tag internal">internal</span> _cache
        :animate: fade-in

        **内部缓存**

    .. dropdown:: <span class="tag attr">attr</span> x
        :animate: fade-in

        **公开属性：x 坐标**

    .. dropdown:: <span class="tag attr">attr</span> y
        :animate: fade-in

        **公开属性：y 坐标**

    .. dropdown:: <span class="tag private">private</span> __normalize() -> None
        :animate: fade-in

        内部归一化计算。

    .. dropdown:: <span class="tag internal">internal</span> _clamp(min_val: float, max_val: float) -> None
        :animate: fade-in

        内部钳制范围。

    .. dropdown:: <span class="tag method">method</span> length() -> float
        :animate: fade-in

        计算向量长度。

        :returns: 向量长度
        :rtype: float

    .. dropdown:: <span class="tag method">method</span> add(other: Type2) -> Type2
        :animate: fade-in

        向量加法。

        :param other: 另一个向量
        :type other: Type2
        :returns: 新向量
        :rtype: Type2

    .. dropdown:: <span class="tag static">static</span> zero() -> Type2
        :animate: fade-in

        静态方法：创建零向量。

        调用方式：``pygame.xxx.Type2.zero()``

        :returns: 零向量 (0, 0)
        :rtype: Type2
