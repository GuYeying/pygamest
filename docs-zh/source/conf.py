import os
import sys
sys.path.insert(0, os.path.abspath('../../'))

project = 'pygamest'
copyright = '2026, GuYeying'
author = 'GuYeying'
language = "zh_CN"   # 中文
release = '0.1.0'

extensions = [
    "sphinx.ext.autodoc",
    "sphinx.ext.napoleon",
    "sphinx_autodoc_typehints",
    "myst_parser",
    "sphinx_design",  # 卡片网格核心扩展
]

source_suffix = {
    '.rst': 'restructuredtext',
    '.md': 'markdown',
}
root_doc = 'index'

# 让autodoc的函数、属性、方法，出现在页面TOC里，支持锚点跳转
toc_object_entries = True
toc_object_entries_show_parents = "domain"

# --------------------------
# 侧边栏布局，原生Sphinx写法，兼容性最强
# --------------------------
html_sidebars = {
    "**": ["sidebar-nav-bs", "localtoc.html"]
}

# --------------- 主题配置（只保留稳定可用参数）---------------
html_theme = "pydata_sphinx_theme"
html_theme_options = {
    "collapse_navigation": True,   # ✅ 左侧导航树：支持折叠三角箭头
    "navigation_depth": 3,
}
# ========== 删掉 html_sidebars 这一段！！不要写！！ ==========


html_static_path = ['_static']

html_context = {
    "display_github": True,
    "github_user": "GuYeying",
    "github_repo": "pygamest",
    "github_version": "main",
    "conf_py_path": "/docs-zh/source/",
}

# ✅ 开启myst支持sphinx-design的网格、卡片语法
myst_enable_extensions = [
    "colon_fence",
    "deflist",
    "attrs_block",
]


# 主题
html_theme = "sphinx_rtd_theme"
html_static_path = ["_static"]
html_css_files = ["custom.css"]