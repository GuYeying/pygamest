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

# myst 标题自动锚点（h1–h5）：URL 可深链到任意成员标题，
# 手写 `{ #id }` 属性语法在 myst 不解析——统一走这里
myst_heading_anchors = 5

# API 页定约跳级（分类 h2 / 类型 h3 / 成员 h4-h5）是有意的，压掉 header 警告
suppress_warnings = ["myst.header"]


html_theme = "sphinx_rtd_theme"
html_css_files = ["custom.css"]