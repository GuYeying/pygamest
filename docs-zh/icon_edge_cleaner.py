#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
图片边缘模糊处理脚本
功能：自动去除背景 + 锐化边缘，输出透明底图，方便你手动裁切
注意：不会分割图片，保留原图完整布局

依赖：pip install Pillow numpy
用法：
  单张图：python icon_edge_cleaner.py input.png -o output.png
  批量：python icon_edge_cleaner.py ./文件夹 -o ./输出文件夹
"""

import os
import argparse
import numpy as np
from PIL import Image, ImageFilter


def remove_background(img, tolerance=35):
    """
    自动去除四角的纯色背景，转为透明
    tolerance: 颜色容差，越大去除范围越广
    """
    img = img.convert("RGBA")
    data = np.array(img)
    r, g, b = data[:, :, 0].astype(float), data[:, :, 1].astype(float), data[:, :, 2].astype(float)

    # 取四个角的平均颜色作为背景参考
    corners = np.array([
        data[0, 0, :3],
        data[0, -1, :3],
        data[-1, 0, :3],
        data[-1, -1, :3],
    ])
    bg = np.mean(corners, axis=0)

    # 计算每个像素与背景色的距离
    diff = np.sqrt((r - bg[0])**2 + (g - bg[1])**2 + (b - bg[2])**2)

    # 距离小于容差的设为透明
    mask = diff < tolerance
    data[mask, 3] = 0

    return Image.fromarray(data)


def sharpen_edges(img, radius=2, percent=180, threshold=2):
    """
    锐化边缘，消除AI生成图的羽化模糊感
    """
    return img.filter(ImageFilter.UnsharpMask(radius=radius, percent=percent, threshold=threshold))


def process_image(input_path, output_path, tolerance=35, sharp_strength=180):
    """处理单张图：去背景 → 锐化 → 保存，保留原图尺寸和布局"""
    img = Image.open(input_path)
    original_size = img.size

    img = remove_background(img, tolerance=tolerance)
    img = sharpen_edges(img, percent=sharp_strength)

    img.save(output_path, "PNG")
    print(f"✓ {os.path.basename(input_path)} → {os.path.basename(output_path)}  尺寸: {original_size}")


def batch_process(input_dir, output_dir, **kwargs):
    """批量处理文件夹内所有图片"""
    os.makedirs(output_dir, exist_ok=True)
    exts = (".png", ".jpg", ".jpeg", ".webp")
    files = [f for f in os.listdir(input_dir) if f.lower().endswith(exts)]

    print(f"找到 {len(files)} 张图片，开始处理...\n")
    for f in files:
        inp = os.path.join(input_dir, f)
        name = os.path.splitext(f)[0]
        out = os.path.join(output_dir, f"{name}_clean.png")
        try:
            process_image(inp, out, **kwargs)
        except Exception as e:
            print(f"✗ {f} 失败: {e}")

    print(f"\n完成！输出目录: {output_dir}")


def main():
    parser = argparse.ArgumentParser(description="图片边缘模糊处理 - 去背景/锐化，方便手动裁切")
    parser.add_argument("input", help="输入图片 或 文件夹")
    parser.add_argument("-o", "--output", help="输出图片 或 文件夹")
    parser.add_argument("-t", "--tolerance", type=int, default=35, help="背景容差 (默认35，背景不均调大)")
    parser.add_argument("-s", "--sharp", type=int, default=180, help="锐化强度 (默认180，模糊严重调大到250)")

    args = parser.parse_args()

    kwargs = {"tolerance": args.tolerance, "sharp_strength": args.sharp}

    if os.path.isdir(args.input):
        out_dir = args.output or os.path.join(args.input, "cleaned")
        batch_process(args.input, out_dir, **kwargs)
    else:
        out_path = args.output or "output_clean.png"
        process_image(args.input, out_path, **kwargs)
        print(f"\n完成！输出: {out_path}")


if __name__ == "__main__":
    main()
