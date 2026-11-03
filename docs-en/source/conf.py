import os
import sys
sys.path.insert(0, os.path.abspath('../../'))

project = 'pygamest'
copyright = '2026, GuYeying'
author = 'GuYeying'
language = "en"
release = '0.1.0'

extensions = [
    "myst_parser",
    "sphinx_rtd_theme",
    "sphinx.ext.autodoc",
    "sphinx_autodoc_typehints",
    "sphinx.ext.viewcode",
    "sphinx_design",  # ✅ 新增：卡片、图标网格组件，就是Arcade首页的核心
]

source_suffix = {
    '.rst': 'restructuredtext',
    '.md': 'markdown',
}
root_doc = 'index'

html_theme = "sphinx_rtd_theme"
html_theme_options = {
    "navigation_depth": 4,
    "collapse_navigation": False,
    "sticky_navigation": True,
}

html_static_path = ['_static']

html_context = {
    "display_github": True,
    "github_user": "GuYeying",
    "github_repo": "pygamest",
    "github_version": "main",
    "conf_py_path": "/docs-en/source/",
}

# ✅ 开启myst支持sphinx-design的网格、卡片语法
myst_enable_extensions = [
    "colon_fence",
    "deflist",
    "attrs_block",
]
