//! pygame.gfxdraw 绑定（2026-10-24 自 native_pygame.rs 拆分；body 原样搬运）。
#![allow(unused_imports)]
use rustpython_derive::{pymodule, pyclass, FromArgs};
use crate::rpy::{bridge, args};
use crate::rpy::pygame::pygame::{PySurface, PySurfaceSession, PyScreen, PyScreenSession, PyColor};
use rustpython_vm::{
    PyObjectRef, Py, PyPayload, PyRef, PyResult, VirtualMachine,
    function::{FuncArgs, PyComparisonValue},
    types::{Comparable, Constructor, PyComparisonOp, Representable},
    builtins::{PyModule, PyStr, PyStrRef, PyType},
};
use std::cell::Cell;
#[pymodule]
    pub mod gfxdraw {
        use crate::rpy::pygame::pygame::{PyScreenSession, PySurface, PySurfaceSession, PyTargetsSession};
        use crate::rpy::args::{SfColor, SfPoints, SfRect};
        use rustpython_derive::FromArgs;
        use crate::rpy::bridge;
        use starfish_pygame::render::DrawTarget;
        use rustpython_vm::{
            PyObjectRef, PyRef, PyResult, VirtualMachine,
            function::{ArgIntoFloat, FuncArgs},
        };

    // ── 动画运行时渲染集成（阶段5：spine / live2d / ozz 数据流经会话
    // batch 上屏——渲染中心在 pygame；各适配库自持管线，底层类型走
    // pygamest）────────────────────────────────────────────────────

    /// mesh(f, surface, positions, uvs, indices, color=None)：任意带贴图
    /// 三角网格通用口——positions/uvs 扁平 f32（uv 纹理域直传），indices
    /// 三角索引按 3 展开；外部动画运行时数据流的消费插座。
    #[pyfunction]
    fn mesh(
        f: PyObjectRef,
        surface: Option<PyRef<PySurface>>,
        positions: Vec<f32>,
        uvs: Vec<f32>,
        indices: Vec<u16>,
        color: Option<SfColor>,
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        let rgba: [f32; 4] = match color {
            Some(c) => [
                c.0[0] as f32 / 255.0,
                c.0[1] as f32 / 255.0,
                c.0[2] as f32 / 255.0,
                c.0[3] as f32 / 255.0,
            ],
            None => [1.0, 1.0, 1.0, 1.0],
        };
        // 纯色路径 = 1×1 白 Surface（draw_mesh 签名必选纹理——回退后形态）
        let white = starfish_pygame::render::Surface::from_rgba8(
            (1, 1),
            &[255, 255, 255, 255],
        );
        let tex: &starfish_pygame::render::Surface = match surface.as_deref() {
            Some(s) => &s.surface,
            None => &white,
        };
        use starfish_pygame::render::DrawTarget;
        match route(&f, vm)? {
            Target::Surf(s) => {
                DrawTarget::draw_mesh(&*s.surface, tex, &positions, &uvs, &indices, rgba)
            }
            Target::Active => bridge::with_session(vm, |t| {
                DrawTarget::draw_mesh(t, tex, &positions, &uvs, &indices, rgba)
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                DrawTarget::draw_mesh(t, tex, &positions, &uvs, &indices, rgba)
            })?,
        }
        Ok(())
    }

    /// draw_spine(f, skeleton, pages)：spine 骨架渲染——chine render()
    /// 命令流 → mesh 上屏（uv 翻 v：page 域 y 向下 → 引擎 v 向上；
    /// positions 为 spine 世界坐标——Auto2d 相机域直入）。pages[cmd.page]
    /// = 图集页 Surface。
    #[cfg(feature = "spine")]
    #[pyfunction]
    fn draw_spine(
        f: PyObjectRef,
        skeleton: PyRef<crate::rpy::pygame_spine::pygame_spine::PySpineSkeleton>,
        pages: Vec<PyRef<PySurface>>,
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        use starfish_pygame::render::DrawTarget;
        let sk = skeleton.inner.borrow();
        let cmds = pygame_spine::render::render(&sk);
        fn draw_cmds<T: DrawTarget + ?Sized>(
            t: &T,
            cmds: &[pygame_spine::RenderCommand],
            pages: &[std::sync::Arc<starfish_pygame::render::Surface>],
        ) {
            for c in cmds {
                let Some(page) = pages.get(c.page) else {
                    continue; // 图集页越界：跳过（数据面校验在 Python 侧）
                };
                let positions: Vec<f32> =
                    c.positions.iter().flat_map(|p| [p.x, p.y]).collect();
                // chine uv = PNG 原生 top-down 域（v=0 图顶）——与引擎纹理
                // 上传方向一致，直传零翻转
                let uvs: Vec<f32> = c.uvs.clone();
                let rgba = [c.color.r, c.color.g, c.color.b, c.color.a];
                t.draw_mesh(page, &positions, &uvs, &c.triangles, rgba);
            }
        }
        let page_surfaces: Vec<std::sync::Arc<starfish_pygame::render::Surface>> =
            pages.iter().map(|p| p.surface.clone()).collect();
        match route(&f, vm)? {
            Target::Surf(s) => {
                draw_cmds(&*s.surface, &cmds, &page_surfaces);
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                draw_cmds(t, &cmds, &page_surfaces);
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                draw_cmds(t, &cmds, &page_surfaces);
                Ok(())
            })?,
        }
    }

    /// masked 双纹理 WGSL：色纹理 × 蒙版纹理 alpha 相乘（Cubism 蒙版
    /// 纹理方案的 GPU 落地面——蒙版区域外 alpha=0 自动透明）。binding
    /// 0 = MVP（蒙版域顶点已烘 NDC——恒等阵）、1/2 = 色纹理+采样、
    /// 3/4 = 蒙版纹理+采样。
    #[cfg(feature = "live2d")]
    const MASKED_WGSL: &str = r#"
struct Mvp {
    m: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> mvp: Mvp;
@group(0) @binding(1) var color_tex: texture_2d<f32>;
@group(0) @binding(2) var color_samp: sampler;
@group(0) @binding(3) var mask_tex: texture_2d<f32>;
@group(0) @binding(4) var mask_samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) muv: vec2<f32>,
}

@vertex
fn vs_main(
    @location(0) p: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) muv: vec2<f32>,
) -> VsOut {
    var o: VsOut;
    o.pos = mvp.m * vec4<f32>(p, 0.0, 1.0);
    o.uv = uv;
    o.muv = muv;
    return o;
}

@fragment
fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(color_tex, color_samp, i.uv);
    let m = textureSample(mask_tex, mask_samp, i.muv);
    return vec4<f32>(c.rgb, c.a * m.a);
}
"#;

    /// 一组蒙版渲染计划（每帧现算——顶点随动画动，span/clip 逐帧刷新；
    /// 坐标域 = scale 烘后的绘制域，与主批次顶点同域）。
    #[cfg(feature = "live2d")]
    struct MaskGroupPlan {
        group: ::pygame_live2d::mask::MaskGroup,
        /// mask ∪ masked 联合包围盒（scale 后）与 clip 仿射（官方
        /// margin 0.25 外扩）——uv = pos*k+c。
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        k_x: f32,
        k_y: f32,
        c_x: f32,
        c_y: f32,
    }

    /// f32 → native-endian bytes（顶点/uniform buffer 装载；零 bytemuck 依赖）
    #[cfg(feature = "live2d")]
    fn f32_bytes(v: &[f32]) -> Vec<u8> {
        v.iter().flat_map(|f| f.to_ne_bytes()).collect()
    }

    /// u16 → native-endian bytes（索引 buffer 装载）
    #[cfg(feature = "live2d")]
    fn u16_bytes(v: &[u16]) -> Vec<u8> {
        v.iter().flat_map(|f| f.to_ne_bytes()).collect()
    }

    /// 蒙版组渲染计划（span/clip 每帧现算——动画驱动顶点位移）。
    #[cfg(feature = "live2d")]
    fn plan_mask_groups(
        meshes: &[::pygame_live2d::moc3::Moc3DrawableMesh],
        scale: f32,
    ) -> Vec<MaskGroupPlan> {
        ::pygame_live2d::mask::mask_groups(meshes)
            .into_iter()
            .map(|group| {
                let (mut min_x, mut min_y, mut max_x, mut max_y) =
                    (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for &i in group.mask_ids.iter().chain(group.masked_ids.iter()) {
                    let Some(m) = meshes.get(i) else {
                        continue;
                    };
                    for v in m.vertices() {
                        let p = v.position();
                        min_x = min_x.min(p[0] * scale);
                        min_y = min_y.min(p[1] * scale);
                        max_x = max_x.max(p[0] * scale);
                        max_y = max_y.max(p[1] * scale);
                    }
                }
                let (k_x, k_y, c_x, c_y) = ::pygame_live2d::mask::mask_clip_transform(
                    min_x, min_y, max_x, max_y, 0.25,
                );
                MaskGroupPlan {
                    group,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    k_x,
                    k_y,
                    c_x,
                    c_y,
                }
            })
            .collect()
    }

    /// 单 drawable 的 clip 域 draw 产物（vbo + bind group + 顶点数）：
    /// 顶点烘蒙版 NDC（ndc = (2u-1, 1-2v)——GPU 渲染到纹理 y 翻转的预
    /// 补偿，采样域 uv=clip 域直读）、uv 双采样（色纹理原生域 + 蒙版
    /// clip 域）。
    #[cfg(feature = "live2d")]
    #[allow(clippy::too_many_arguments)]
    fn clip_draw_build(
        ctx: &mut starfish_pygame::render::OverlayCtx,
        device: &wgpu::Device,
        m: &::pygame_live2d::moc3::Moc3DrawableMesh,
        plan: &MaskGroupPlan,
        color_view: std::sync::Arc<wgpu::TextureView>,
        mask_view: std::sync::Arc<wgpu::TextureView>,
        sampler: &std::sync::Arc<wgpu::Sampler>,
        mvp_buf: &std::sync::Arc<wgpu::Buffer>,
        scale: f32,
    ) -> (
        std::sync::Arc<wgpu::Buffer>,
        std::sync::Arc<wgpu::Buffer>,
        wgpu::BindGroup,
        u32,
    ) {
        use wgpu::util::DeviceExt;
        // 顶点先烘 scale（plan 的 span/k/c 全在 scale 域——与主批次
        // 顶点同域；漏乘 = u/w 飘出 [0,1] 画到界外）
        let verts: Vec<f32> = m
            .vertices()
            .iter()
            .flat_map(|v| {
                let p = v.position();
                let u = p[0] * scale * plan.k_x + plan.c_x;
                let w = p[1] * scale * plan.k_y + plan.c_y;
                [u * 2.0 - 1.0, 1.0 - w * 2.0, v.uv()[0], v.uv()[1], u, w]
            })
            .collect();
        let vbo = std::sync::Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("pygamest_l2d_clip_vbo"),
                contents: &f32_bytes(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));
        // mocari 顶点数组 = 索引化三角形池——必须按索引 draw（非索引
        // 画池序 = 退化三角形满天飞、形状全丢）
        let ibo = std::sync::Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("pygamest_l2d_clip_ibo"),
                contents: &u16_bytes(m.indices()),
                usage: wgpu::BufferUsages::INDEX,
            },
        ));
        let bg = ctx
            .access()
            .bind_group_builder()
            .uniform_raw(0, mvp_buf.clone(), 64)
            .texture_view(1, color_view)
            .sampler(2, sampler.clone())
            .texture_view(3, mask_view)
            .sampler(4, sampler.clone())
            .build(Some("pygamest_l2d_clip_bg"))
            .bind_group()
            .clone();
        let n = m.indices().len() as u32;
        (vbo, ibo, bg, n)
    }

    /// 一批 clip draw 的管线重放（蒙版/masked 两段 pass 闭包共用形态）。
    #[cfg(feature = "live2d")]
    fn clip_draw_replay(
        rp: &mut wgpu::RenderPass,
        pipeline: &wgpu::RenderPipeline,
        draws: &[(
            std::sync::Arc<wgpu::Buffer>,
            std::sync::Arc<wgpu::Buffer>,
            wgpu::BindGroup,
            u32,
        )],
    ) {
        rp.set_pipeline(pipeline);
        for (vbo, ibo, bg, n) in draws {
            rp.set_bind_group(0, bg, &[]);
            rp.set_vertex_buffer(0, vbo.slice(..));
            rp.set_index_buffer(ibo.slice(..), wgpu::IndexFormat::Uint16);
            rp.draw_indexed(0..*n, 0, 0..1);
        }
    }

    /// 蒙版/masked 两段 overlay 直绘（Cubism 蒙版纹理方案 GPU 落地）：
    /// 每组先把 mask 来源 drawable 白形画入 Mg（512² 离屏）、再把
    /// masked drawable 双纹理（色 × Mg alpha）画入 C——**立即 overlay**
    /// 先于主批次提交，主批次 draw_mesh 合成 C 同帧采样零错帧。
    #[cfg(feature = "live2d")]
    fn masked_overlay_passes(
        ctx: &mut starfish_pygame::render::OverlayCtx,
        plans: &[MaskGroupPlan],
        meshes: &[::pygame_live2d::moc3::Moc3DrawableMesh],
        tex_list: &[std::sync::Arc<starfish_pygame::render::Surface>],
        mask_pool: &[std::sync::Arc<starfish_pygame::render::Surface>],
        color_pool: &[std::sync::Arc<starfish_pygame::render::Surface>],
        pipeline: &std::sync::Arc<wgpu::RenderPipeline>,
        sampler: &std::sync::Arc<wgpu::Sampler>,
        white: &std::sync::Arc<starfish_pygame::render::Surface>,
        scale: f32,
    ) {
        use wgpu::util::DeviceExt;
        let device = ctx.device().clone();
        // 恒等 MVP（clip 域顶点已烘 NDC——直绘）
        let idm: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let mvp_buf = std::sync::Arc::new(device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("pygamest_l2d_mask_mvp"),
                contents: &f32_bytes(&idm),
                usage: wgpu::BufferUsages::UNIFORM,
            },
        ));
        for (gi, plan) in plans.iter().enumerate() {
            let (Some(mg), Some(csurf)) = (mask_pool.get(gi), color_pool.get(gi)) else {
                continue;
            };
            let mask_view = mg.texture_view();
            let white_view = white.texture_view();
            // ① 蒙版 pass：mask 白形画入 Mg（蒙版面本身是本 pass 渲染
            // 目标——自采样会 usage 冲突，蒙版槽以白纹理代之 = 恒等裁剪）
            let mut mask_draws = Vec::new();
            for &di in &plan.group.mask_ids {
                let Some(m) = meshes.get(di) else {
                    continue;
                };
                mask_draws.push(clip_draw_build(
                    ctx,
                    &device,
                    m,
                    plan,
                    white_view.clone(),
                    white_view.clone(),
                    sampler,
                    &mvp_buf,
                    scale,
                ));
            }
            let (pl0, md) = (pipeline.clone(), mask_draws);
            ctx.pass_on_surface("pygamest_l2d_mask", mg, wgpu::Color::TRANSPARENT, move |rp| {
                clip_draw_replay(rp.inner_mut(), &pl0, &md);
            });
            // ② masked pass：色 × Mg alpha 画入 C
            let mut masked_draws = Vec::new();
            for &di in &plan.group.masked_ids {
                let Some(m) = meshes.get(di) else {
                    continue;
                };
                let Some(tex) = tex_list.get(m.texture_index() as usize) else {
                    continue;
                };
                masked_draws.push(clip_draw_build(
                    ctx,
                    &device,
                    m,
                    plan,
                    tex.texture_view(),
                    mask_view.clone(),
                    sampler,
                    &mvp_buf,
                    scale,
                ));
            }
            let (pl1, mds) = (pipeline.clone(), masked_draws);
            ctx.pass_on_surface(
                "pygamest_l2d_masked",
                csurf,
                wgpu::Color::TRANSPARENT,
                move |rp| {
                    clip_draw_replay(rp.inner_mut(), &pl1, &mds);
                },
            );
        }
    }

    /// draw_live2d(f, model)：Live2D 模型渲染（含蒙版纹理三段编排）：
    /// ① 蒙版/masked 段（立即 overlay——Mg 白形 + C 双纹理，独立 WGSL
    /// 管线）；② 主批次 draw_mesh：普通 drawable 直绘 + masked 组在
    /// render_order 位以 C 面 quad 合成（alpha 混合，蒙版区域外 C.a=0
    /// 自动透明——层级随渲染序、零错帧）。
    #[cfg(feature = "live2d")]
    #[derive(FromArgs)]
    struct Live2DDrawArgs {
        model: PyRef<crate::rpy::pygame_live2d::pygame_live2d::PyLive2DRuntime>,
        #[pyarg(any, optional)]
        scale: Option<ArgIntoFloat>,
    }

    #[cfg(feature = "live2d")]
    #[pyfunction]
    fn draw_live2d(f: PyObjectRef, args: Live2DDrawArgs, vm: &VirtualMachine) -> PyResult<()> {
        let model = args.model;
        let scale = args.scale.map(|v| v.into_float() as f32).unwrap_or(1.0);
        use starfish_pygame::render::{DrawTarget, OverlayCtx, Surface as SfSurface};
        // 纹理惰性构建（一次——2048² RGBA 每帧重建不可接受）
        {
            let mut tc = model.textures.borrow_mut();
            if tc.is_none() {
                let rt = model.inner.borrow();
                let surfaces: Vec<std::sync::Arc<SfSurface>> = rt
                    .textures()
                    .iter()
                    .map(|t| {
                        std::sync::Arc::new(SfSurface::from_rgba8(
                            (t.width(), t.height()),
                            t.rgba(),
                        ))
                    })
                    .collect();
                *tc = Some(surfaces);
            }
            // 白 1×1 纯色面（mask 白形 + masked 段恒等蒙版槽）
            if model.white_1x1.borrow().is_none() {
                *model.white_1x1.borrow_mut() = Some(std::sync::Arc::new(
                    SfSurface::from_rgba8((1, 1), &[255, 255, 255, 255]),
                ));
            }
            // masked 双纹理管线惰性构建（一次；蒙版/masked 两段共用；
            // 离屏面恒 Rgba8Unorm——目标格式定死）
            if model.masked_pipeline.borrow().is_none() {
                let gpu = starfish_pygame::render::gpu_info().ok_or_else(|| {
                    vm.new_runtime_error("Live2D 蒙版渲染需先 set_mode 建立 GPU 会话")
                })?;
                let module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("pygamest_live2d_masked"),
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(MASKED_WGSL)),
                });
                let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("pygamest_live2d_masked_samp"),
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    ..Default::default()
                });
                let bgl = gpu
                    .device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("pygamest_live2d_masked_bgl"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                // 构建器 uniform_raw 恒全阶段（V|F|C）——
                                // 管线侧 BGL 须同声明（兼容性校验按位比对）
                                visibility: wgpu::ShaderStages::VERTEX
                                    | wgpu::ShaderStages::FRAGMENT
                                    | wgpu::ShaderStages::COMPUTE,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: wgpu::BufferSize::new(64),
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 2,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 3,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 4,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                    });
                let pl = gpu
                    .device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("pygamest_live2d_masked_pl"),
                        layout: Some(
                            &gpu.device
                                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                                    label: Some("pygamest_live2d_masked_layout"),
                                    bind_group_layouts: &[Some(&bgl)],
                                    immediate_size: 0,
                                }),
                        ),
                        vertex: wgpu::VertexState {
                            module: &module,
                            entry_point: Some("vs_main"),
                            compilation_options: Default::default(),
                            buffers: &[Some(wgpu::VertexBufferLayout {
                                array_stride: 24,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &[
                                    wgpu::VertexAttribute {
                                        format: wgpu::VertexFormat::Float32x2,
                                        offset: 0,
                                        shader_location: 0,
                                    },
                                    wgpu::VertexAttribute {
                                        format: wgpu::VertexFormat::Float32x2,
                                        offset: 8,
                                        shader_location: 1,
                                    },
                                    wgpu::VertexAttribute {
                                        format: wgpu::VertexFormat::Float32x2,
                                        offset: 16,
                                        shader_location: 2,
                                    },
                                ],
                            })],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &module,
                            entry_point: Some("fs_main"),
                            compilation_options: Default::default(),
                            targets: &[Some(wgpu::ColorTargetState {
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                        }),
                        primitive: wgpu::PrimitiveState::default(),
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState::default(),
                        multiview_mask: None,
                        cache: None,
                    });
                *model.masked_sampler.borrow_mut() = Some(std::sync::Arc::new(sampler));
                *model.masked_bgl.borrow_mut() = Some(std::sync::Arc::new(bgl));
                *model.masked_pipeline.borrow_mut() = Some(std::sync::Arc::new(pl));
            }
        }
        let pl = model
            .masked_pipeline
            .borrow()
            .clone()
            .ok_or_else(|| vm.new_runtime_error("masked 管线未初始化"))?;
        let sampler = model
            .masked_sampler
            .borrow()
            .clone()
            .ok_or_else(|| vm.new_runtime_error("masked 采样器未初始化"))?;
        let white = model
            .white_1x1
            .borrow()
            .clone()
            .ok_or_else(|| vm.new_runtime_error("白面未初始化"))?;
        let texs = model.textures.borrow();
        let tex_list = texs.as_ref().expect("纹理缓存刚确保");
        let rt = model.inner.borrow();
        // 按 Cubism RenderOrder 排序（drawable 索引序 ≠ 绘制序——运行时
        // 顺序动态变化；不排序 = 手臂/头发层级错乱、该在背后的画到前面）
        let mut ordered: Vec<_> = rt.runtime().meshes().iter().enumerate().collect();
        ordered.sort_by_key(|(_, m)| m.render_order());
        // 蒙版组计划（每帧现算——span/clip 随动画刷新）+ Mg/C 面池扩容
        let plans = plan_mask_groups(rt.runtime().meshes(), scale);
        {
            let mut mp = model.mask_pool.borrow_mut();
            while mp.len() < plans.len() {
                mp.push(std::sync::Arc::new(SfSurface::new((512, 512))));
            }
            let mut cp = model.color_pool.borrow_mut();
            while cp.len() < plans.len() {
                cp.push(std::sync::Arc::new(SfSurface::new((512, 512))));
            }
        }
        fn draw_meshes<T: DrawTarget + ?Sized>(
            t: &T,
            meshes: &[(usize, &::pygame_live2d::moc3::Moc3DrawableMesh)],
            tex_list: &[std::sync::Arc<SfSurface>],
            scale: f32,
            plans: &[MaskGroupPlan],
            color_pool: &[std::sync::Arc<SfSurface>],
        ) {
            let mut composited: Vec<usize> = Vec::new();
            for (di, m) in meshes {
                // masked drawable → C 面 quad 在组首绘位合成（层级随
                // render_order 落位；同组其余 masked 不重复合成）
                if !m.masks().is_empty() {
                    let Some((gi, plan)) = plans
                        .iter()
                        .enumerate()
                        .find(|(_, p)| p.group.masked_ids.contains(di))
                    else {
                        continue;
                    };
                    if composited.contains(&gi) {
                        continue;
                    }
                    composited.push(gi);
                    let Some(csurf) = color_pool.get(gi) else {
                        continue;
                    };
                    let positions = [
                        plan.min_x, plan.min_y, plan.max_x, plan.min_y, plan.max_x, plan.max_y,
                        plan.min_x, plan.max_y,
                    ];
                    // uv 直读 clip 域（蒙版段顶点烘 NDC 时已预补偿 y 翻转
                    // ——纹理 v == clip v，此处零翻转）
                    let uvs = [0.0f32, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
                    let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
                    t.draw_mesh(csurf, &positions, &uvs, &indices, [1.0, 1.0, 1.0, 1.0]);
                    continue;
                }
                let tex_i = m.texture_index();
                let Some(tex) = tex_list.get(tex_i as usize) else {
                    continue;
                };
                // mocari 顶点已是归一化 y 向上域——直传零翻转
                let positions: Vec<f32> = m
                    .vertices()
                    .iter()
                    .flat_map(|v| [v.position()[0] * scale, v.position()[1] * scale])
                    .collect();
                // mocari uv = 纹理原生域直传（零翻转）
                let uvs: Vec<f32> = m
                    .vertices()
                    .iter()
                    .flat_map(|v| [v.uv()[0], v.uv()[1]])
                    .collect();
                let opacity = m.opacity();
                let indices = m.indices().clone();
                let rgba = [1.0, 1.0, 1.0, opacity.clamp(0.0, 1.0)];
                t.draw_mesh(tex, &positions, &uvs, &indices, rgba);
            }
        }
        match route(&f, vm)? {
            Target::Surf(s) => {
                if !plans.is_empty() {
                    s.surface.with_overlay(|ctx| {
                        masked_overlay_passes(
                            ctx,
                            &plans,
                            rt.runtime().meshes(),
                            tex_list,
                            &model.mask_pool.borrow(),
                            &model.color_pool.borrow(),
                            &pl,
                            &sampler,
                            &white,
                            scale,
                        );
                    });
                }
                draw_meshes(
                    &*s.surface,
                    &ordered,
                    tex_list,
                    scale,
                    &plans,
                    &model.color_pool.borrow(),
                );
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                if !plans.is_empty() {
                    t.with_overlay(|ctx| {
                        masked_overlay_passes(
                            ctx,
                            &plans,
                            rt.runtime().meshes(),
                            tex_list,
                            &model.mask_pool.borrow(),
                            &model.color_pool.borrow(),
                            &pl,
                            &sampler,
                            &white,
                            scale,
                        );
                    });
                }
                draw_meshes(
                    t,
                    &ordered,
                    tex_list,
                    scale,
                    &plans,
                    &model.color_pool.borrow(),
                );
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                // MRT v1 无 overlay（引擎限）——masked 跳过定约沿用
                draw_meshes(t, &ordered, tex_list, scale, &[], &[]);
                Ok(())
            })?,
        }
    }

    /// draw_ozz_quads(f, player, scale=1.0, cx=0.0, cy=0.0, size=14.0)：
    /// ozz 关节盒体动画渲染——每骨一个 quad（世界位 + x/y 基方向定向，
    /// 单次 draw_mesh 纯色路径 None 纹理批量上屏——67 关节 = 单 draw）。
    #[cfg(feature = "ozz")]
    #[derive(FromArgs)]
    struct OzzQuadsArgs {
        player: PyRef<crate::rpy::pygame_ozz::pygame_ozz::PyOzzPlayer>,
        #[pyarg(any, optional)]
        scale: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        cx: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        cy: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        size: Option<ArgIntoFloat>,
    }

    #[cfg(feature = "ozz")]
    #[pyfunction]
    fn draw_ozz_quads(f: PyObjectRef, args: OzzQuadsArgs, vm: &VirtualMachine) -> PyResult<()> {
        use starfish_pygame::render::{BatchTopology, DrawTarget};
        let s = args.scale.map(|v| v.into_float() as f32).unwrap_or(1.0);
        let ox = args.cx.map(|v| v.into_float() as f32).unwrap_or(0.0);
        let oy = args.cy.map(|v| v.into_float() as f32).unwrap_or(0.0);
        // 盒体宽 = 屏域单位（与骨骼缩放解耦——骨骼域随资产变化）
        let hw = args
            .size
            .map(|v| v.into_float() as f32)
            .unwrap_or(14.0)
            * 0.5;
        let pos = args.player.inner.borrow().joint_positions_xy();
        let parents = args.player.inner.borrow().joint_parents();
        // 盒体沿"父→关节"骨段方向定向（关节自身基向量随动画旋转剧烈，
        // 骨段方向才是可读的肢体朝向）；根关节画小方块。
        let mut positions: Vec<f32> = Vec::with_capacity(pos.len() * 8);
        let mut uvs: Vec<f32> = Vec::with_capacity(pos.len() * 8);
        let mut indices: Vec<u16> = Vec::with_capacity(pos.len() * 6);
        let pt = |i: usize| -> (f32, f32) {
            let (x, y) = pos[i];
            (x * s + ox, y * s + oy)
        };
        for (i, parent) in parents.iter().enumerate() {
            let (jx, jy) = pt(i);
            let corners: [(f32, f32); 4] = if *parent >= 0 {
                let Some(ppos) = pos.get(*parent as usize) else {
                    continue;
                };
                let (px_, py_) = pt(*parent as usize);
                let (dx, dy) = (jx - px_, jy - py_);
                let len = (dx * dx + dy * dy).sqrt().max(1e-6);
                let (dx, dy) = (dx / len, dy / len);
                let (per_x, per_y) = (-dy, dx);
                [
                    (jx + per_x * hw, jy + per_y * hw),
                    (jx - per_x * hw, jy - per_y * hw),
                    (px_ - per_x * hw, py_ - per_y * hw),
                    (px_ + per_x * hw, py_ + per_y * hw),
                ]
            } else {
                [
                    (jx - hw, jy - hw),
                    (jx + hw, jy - hw),
                    (jx + hw, jy + hw),
                    (jx - hw, jy + hw),
                ]
            };
            for (cx0, cy0) in corners {
                positions.push(cx0);
                positions.push(cy0);
            }
            uvs.extend_from_slice(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]);
            let b = i as u16 * 4;
            indices.extend_from_slice(&[b, b + 1, b + 2, b + 2, b + 3, b]);
        }
        fn draw_quads<T: DrawTarget + ?Sized>(
            t: &T,
            positions: &[f32],
            uvs: &[f32],
            indices: &[u16],
            white: &starfish_pygame::render::Surface,
        ) {
            t.draw_mesh(white, positions, uvs, indices, [1.0, 0.85, 0.45, 0.92]);
        }
        let white_q = starfish_pygame::render::Surface::from_rgba8((1, 1), &[255, 255, 255, 255]);
        match route(&f, vm)? {
            Target::Surf(s2) => {
                draw_quads(&*s2.surface, &positions, &uvs, &indices, &white_q);
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                draw_quads(t, &positions, &uvs, &indices, &white_q);
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                draw_quads(t, &positions, &uvs, &indices, &white_q);
                Ok(())
            })?,
        }
    }

    /// draw_ozz_bones(f, player, scale=1.0, cx=0.0, cy=0.0)：ozz 骨架骨骼
    /// 线渲染（父子世界位 xy × scale + 中心偏移——Auto2d 中心系直入；
    /// ozz y 向上与引擎同构零翻转。蒙皮 mesh 面落地前的骨骼可视化形态）。
    #[cfg(feature = "ozz")]
    #[derive(FromArgs)]
    struct OzzBonesArgs {
        player: PyRef<crate::rpy::pygame_ozz::pygame_ozz::PyOzzPlayer>,
        #[pyarg(any, optional)]
        scale: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        cx: Option<ArgIntoFloat>,
        #[pyarg(any, optional)]
        cy: Option<ArgIntoFloat>,
    }

    #[cfg(feature = "ozz")]
    #[pyfunction]
    fn draw_ozz_bones(f: PyObjectRef, args: OzzBonesArgs, vm: &VirtualMachine) -> PyResult<()> {
        let (player, scale, cx, cy) = (args.player, args.scale, args.cx, args.cy);
        use starfish_pygame::render::BatchTopology;
        let s = scale.map(|v| v.into_float() as f32).unwrap_or(1.0);
        let ox = cx.map(|v| v.into_float() as f32).unwrap_or(0.0);
        let oy = cy.map(|v| v.into_float() as f32).unwrap_or(0.0);
        let pos = player.inner.borrow().joint_positions_xy();
        let parents = player.inner.borrow().joint_parents();
        fn draw_bones<T: starfish_pygame::render::DrawTarget + ?Sized>(
            t: &T,
            pos: &[(f32, f32)],
            parents: &[i16],
            s: f32,
            ox: f32,
            oy: f32,
        ) {
            let px = |x: f32| (x * s + ox) as i32;
            let py = |y: f32| (y * s + oy) as i32;
            for (i, parent) in parents.iter().enumerate() {
                if *parent < 0 {
                    continue; // 根关节无父
                }
                let Some(ppos) = pos.get(*parent as usize) else {
                    continue;
                };
                let (jx, jy) = pos[i];
                t.push_points(
                    BatchTopology::Lines,
                    None,
                    &[(px(jx) as f32, py(jy) as f32), (px(ppos.0) as f32, py(ppos.1) as f32)],
                    [1.0, 0.9, 0.3, 1.0],
                );
            }
        }
        match route(&f, vm)? {
            Target::Surf(s2) => {
                draw_bones(&*s2.surface, &pos, &parents, s, ox, oy);
                Ok(())
            }
            Target::Active => bridge::with_session(vm, |t| {
                draw_bones(t, &pos, &parents, s, ox, oy);
                Ok(())
            })?,
            Target::Targets => bridge::with_mrt_session(vm, |t| {
                draw_bones(t, &pos, &parents, s, ox, oy);
                Ok(())
            })?,
        }
    }


        /// gfxdraw 目标：SurfaceSession 直绘（自有 batch）；Screen 会话
        /// （PyScreenSession）= 当前活跃会话（with_session 路由，须在 with 内）。
        pub(crate) enum Target<'a> {
            Surf(&'a PySurfaceSession),
            Active,
            /// MRT 活跃槽（with_mrt_session——镜像双写会话）
            Targets,
        }


        pub(crate) fn route<'a>(f: &'a PyObjectRef, vm: &VirtualMachine) -> PyResult<Target<'a>> {
            if let Some(ss) = f.downcast_ref::<PySurfaceSession>() {
                return Ok(Target::Surf(ss));
            }
            if f.downcast_ref::<PyScreenSession>().is_some() {
                return Ok(Target::Active);
            }
            if f.downcast_ref::<crate::rpy::pygame::pygame::PyTargetsSession>().is_some() {
                return Ok(Target::Targets);
            }
            Err(vm.new_type_error(
                "gfxdraw 目标应为渲染会话（with ...render() as f 的 f）",
            ))
        }

        // ── 渲染统一 draw 面：状态机口（相机/变换随 gfx 管线分域；
        // renderpass 作首参——与 SDL_gfx 签名同构，render 会话保持纯通道）──

        /// set_camera(f, camera)：会话相机切换（后续 gfx 绘制进新段；
        /// 仅本会话生效）
        #[pyfunction]
        fn set_camera(
            f: PyObjectRef,
            camera: PyRef<crate::rpy::pygame::render::render::PyCamera>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.with_batch(|b| b.set_mvp(Some(camera.0.mvp()))),
                Target::Active => bridge::with_session(vm, |t| t.set_camera(&camera.0))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.set_camera(&camera.0))?,
            }
            Ok(())
        }

        /// reset_camera(f)：回 Auto 兜底相机（按目标尺寸现算）
        #[pyfunction]
        fn reset_camera(f: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.with_batch(|b| b.reset_mvp()),
                Target::Active => bridge::with_session(vm, |t| t.reset_camera())?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.reset_camera())?,
            }
            Ok(())
        }

        /// set_transform(f, model)：会话变换（后续绘制顶点按此矩阵 CPU 烘）
        #[pyfunction]
        fn set_transform(
            f: PyObjectRef,
            model: PyRef<crate::rpy::pygame::math::math::PyMat4>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.with_batch(|b| b.set_model(model.0.get())),
                Target::Active => bridge::with_session(vm, |t| t.set_transform(model.0.get()))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.set_transform(model.0.get()))?,
            }
            Ok(())
        }


        /// reset_transform(f)：回恒等变换
        #[pyfunction]
        fn reset_transform(f: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.with_batch(|b| b.reset_model()),
                Target::Active => bridge::with_session(vm, |t| t.reset_transform())?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.reset_transform())?,
            }
            Ok(())
        }

        /// set_color(f)：one-shot 绘制色（下一次 gfxdraw 图元用此色；
        /// draw 后自动重置回默认紫色）
        #[pyfunction]
        fn set_color(
            f: PyObjectRef,
            color: SfColor,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            let c = [
                color.0[0] as f32 / 255.0,
                color.0[1] as f32 / 255.0,
                color.0[2] as f32 / 255.0,
                color.0[3] as f32 / 255.0,
            ];
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.set_draw_color(c),
                Target::Active => bridge::with_session(vm, |t| t.set_draw_color(c))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.set_draw_color(c))?,
            }
            Ok(())
        }

        /// set_wireframe(f, enabled=True)：one-shot 线框模式（下一次图元
        /// 渲染为三角网格边线——呈现原始三角化的边）
        #[pyfunction]
        fn set_wireframe(
            f: PyObjectRef,
            args: FuncArgs,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            use rustpython_vm::convert::TryFromObject;
            let enabled = args.args.first()
                .and_then(|o| bool::try_from_object(vm, o.clone()).ok())
                .unwrap_or(true);
            match route(&f, vm)? {
                Target::Surf(s) => s.surface.set_draw_wireframe(enabled),
                Target::Active => bridge::with_session(vm, |t| t.set_draw_wireframe(enabled))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| t.set_draw_wireframe(enabled))?,
            }
            Ok(())
        }

        /// draw_surface(f, src)：画离屏面（位置/变换由 set_transform 承载）
        #[pyfunction]
        fn draw_surface(f: PyObjectRef, src: PyRef<PySurface>, vm: &VirtualMachine) -> PyResult<()> {
            use starfish_pygame::render::DrawTarget;
            match route(&f, vm)? {
                Target::Surf(s) => DrawTarget::draw_surface(&*s.surface, &src.surface),
                Target::Active => bridge::with_session(vm, |t| DrawTarget::draw_surface(t, &src.surface))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| DrawTarget::draw_surface(t, &src.surface))?,
            }
            Ok(())
        }

        /// draw_surface_centered(f, src, center)：中心锚画面
        #[pyfunction]
        fn draw_surface_centered(
            f: PyObjectRef,
            src: PyRef<PySurface>,
            center: crate::rpy::args::SfPoint,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            use starfish_pygame::render::DrawTarget;
            match route(&f, vm)? {
                Target::Surf(s) => DrawTarget::draw_surface_centered(
                    &*s.surface, &src.surface, (center.0 as f32, center.1 as f32)),
                Target::Active => bridge::with_session(vm, |t| DrawTarget::draw_surface_centered(
                    t, &src.surface, (center.0 as f32, center.1 as f32)))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| DrawTarget::draw_surface_centered(
                    t, &src.surface, (center.0 as f32, center.1 as f32)))?,
            }
            Ok(())
        }

        /// draw_surface_uv(f, src, uv, tint=None)：子区域画面（area 语义继任）
        #[pyfunction]
        fn draw_surface_uv(
            f: PyObjectRef,
            src: PyRef<PySurface>,
            uv: Vec<f32>,
            tint: Option<SfColor>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            use starfish_pygame::render::DrawTarget;
            match route(&f, vm)? {
                Target::Surf(s) => DrawTarget::draw_surface_uv(
                    &*s.surface, &src.surface,
                    [uv[0], uv[1], uv[2], uv[3]],
                    tint.map(|c| c.0).unwrap_or([255, 255, 255, 255])),
                Target::Active => bridge::with_session(vm, |t| DrawTarget::draw_surface_uv(
                    t, &src.surface,
                    [uv[0], uv[1], uv[2], uv[3]],
                    tint.map(|c| c.0).unwrap_or([255, 255, 255, 255])))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| DrawTarget::draw_surface_uv(
                    t, &src.surface,
                    [uv[0], uv[1], uv[2], uv[3]],
                    tint.map(|c| c.0).unwrap_or([255, 255, 255, 255])))?,
            }
            Ok(())
        }

        /// draw_surface_tf(f, src, mat, tint=None)：显式矩阵画面（mat 全权，
        /// 不复合会话 transform）
        #[pyfunction]
        fn draw_surface_tf(
            f: PyObjectRef,
            src: PyRef<PySurface>,
            mat: PyRef<crate::rpy::pygame::math::math::PyMat4>,
            tint: Option<SfColor>,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            use starfish_pygame::render::DrawTarget;
            let tint = tint.map(|c| c.0).map(|px| [px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0, px[3] as f32 / 255.0]);
            match route(&f, vm)? {
                Target::Surf(s) => DrawTarget::draw_surface_tf(
                    &*s.surface, &src.surface, mat.0.get(), tint),
                Target::Active => bridge::with_session(vm, |t| DrawTarget::draw_surface_tf(
                    t, &src.surface, mat.0.get(), tint))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| DrawTarget::draw_surface_tf(
                    t, &src.surface, mat.0.get(), tint))?,
            }
            Ok(())
        }

        /// pixel(f, x, y)
        #[pyfunction]
        fn pixel(f: PyObjectRef, x: i32, y: i32, vm: &VirtualMachine) -> PyResult<()> {
                match route(&f, vm)? {
                Target::Surf(s) => {
                    starfish_pygame::draw::rect(&*s.surface,
                        [x, y, 1, 1], 0)
                }
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::rect(t,
                        [x, y, 1, 1], 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::rect(t,
                        [x, y, 1, 1], 0)
                })?,
            };
            Ok(())
        }

        /// hline(f, x1, x2, y)
        #[pyfunction]
        fn hline(
            f: PyObjectRef,
            x1: i32,
            x2: i32,
            y: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            line(f, x1, y, x2, y, vm)
        }

        /// vline(f, x, y1, y2)
        #[pyfunction]
        fn vline(
            f: PyObjectRef,
            x: i32,
            y1: i32,
            y2: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            line(f, x, y1, x, y2, vm)
        }

        /// line(f, x1, y1, x2, y2)
        #[pyfunction]
        fn line(
            f: PyObjectRef,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::line(&*s.surface, (x1, y1), (x2, y2), 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::line(t, (x1, y1), (x2, y2), 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::line(t, (x1, y1), (x2, y2), 1)
                })?,
            };
            Ok(())
        }

        /// aaline(f, x1, y1, x2, y2)：抗锯齿线 = 兼容别名
        ///（AA 由 wgpu 线性采样承载，同实现）。
        #[pyfunction]
        fn aaline(
            f: PyObjectRef,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            line(f, x1, y1, x2, y2, vm)
        }

        /// circle(f, x, y, radius)：1px 描边圆。
        #[pyfunction]
        fn circle(
            f: PyObjectRef,
            x: i32,
            y: i32,
            radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let r = radius.max(0) as u32;
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::circle(&*s.surface, (x, y), r, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::circle(t, (x, y), r, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::circle(t, (x, y), r, 1)
                })?,
            };
            Ok(())
        }

        /// aacircle(f, x, y, radius)：抗锯齿圆 = 兼容别名。
        #[pyfunction]
        fn aacircle(
            f: PyObjectRef,
            x: i32,
            y: i32,
            radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            circle(f, x, y, radius, vm)
        }

        /// filled_circle(f, x, y, radius)：填充圆。
        #[pyfunction]
        fn filled_circle(
            f: PyObjectRef,
            x: i32,
            y: i32,
            radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let r = radius.max(0) as u32;
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::circle(&*s.surface, (x, y), r, 0),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::circle(t, (x, y), r, 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::circle(t, (x, y), r, 0)
                })?,
            };
            Ok(())
        }

        /// ellipse(f, x, y, rx, ry)：1px 描边椭圆（中心 + 半径轴）。
        #[pyfunction]
        fn ellipse(
            f: PyObjectRef,
            x: i32,
            y: i32,
            rx: i32,
            ry: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let rect = [x - rx, y - ry, rx.max(0) * 2, ry.max(0) * 2];
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::ellipse(&*s.surface, rect, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::ellipse(t, rect, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::ellipse(t, rect, 1)
                })?,
            };
            Ok(())
        }

        /// aaellipse(f, x, y, rx, ry)：抗锯齿椭圆 = 兼容别名。
        #[pyfunction]
        fn aaellipse(
            f: PyObjectRef,
            x: i32,
            y: i32,
            rx: i32,
            ry: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            ellipse(f, x, y, rx, ry, vm)
        }

        /// filled_ellipse(f, x, y, rx, ry)：填充椭圆。
        #[pyfunction]
        fn filled_ellipse(
            f: PyObjectRef,
            x: i32,
            y: i32,
            rx: i32,
            ry: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let rect = [x - rx, y - ry, rx.max(0) * 2, ry.max(0) * 2];
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::ellipse(&*s.surface, rect, 0),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::ellipse(t, rect, 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::ellipse(t, rect, 0)
                })?,
            };
            Ok(())
        }

        /// arc(f, x, y, radius, start_angle, stop_angle)：椭圆弧
        ///（**度数制**——SDL_gfx 惯例；1px 描边，v1 简化）。
        #[pyfunction]
        fn arc(
            f: PyObjectRef,
            x: i32,
            y: i32,
            radius: i32,
            start_angle: i32,
            stop_angle: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let rect = [x - radius, y - radius, radius.max(0) * 2, radius.max(0) * 2];
            let (s, e) = (
                (start_angle as f32).to_radians(),
                (stop_angle as f32).to_radians(),
            );
            match route(&f, vm)? {
                Target::Surf(sfc) => starfish_pygame::draw::arc(&*sfc.surface, rect, s, e, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::arc(t, rect, s, e, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::arc(t, rect, s, e, 1)
                })?,
            };
            Ok(())
        }

        /// trigon 坐标组（9 参超 RP 原生函数实现上限——FromArgs 结构体
        /// 按声明序扁平展开，Python 签名不变）
        #[derive(rustpython_derive::FromArgs)]
        pub(crate) struct TrigonArgs {
            #[pyarg(any)]
            pub(crate) x1: i32,
            #[pyarg(any)]
            pub(crate) y1: i32,
            #[pyarg(any)]
            pub(crate) x2: i32,
            #[pyarg(any)]
            pub(crate) y2: i32,
            #[pyarg(any)]
            pub(crate) x3: i32,
            #[pyarg(any)]
            pub(crate) y3: i32,
        }

        /// trigon(f, x1, y1, x2, y2, x3, y3)：1px 描边三角。
        #[pyfunction]
        fn trigon(f: PyObjectRef, args: TrigonArgs, vm: &VirtualMachine) -> PyResult<()> {
            filled_polygon3(&f, args.x1, args.y1, args.x2, args.y2, args.x3, args.y3, vm)?;
            // 描边三角 = 填充 + 1px 轮廓（gfxdraw trigon 语义）
            polygon3(&f, args.x1, args.y1, args.x2, args.y2, args.x3, args.y3, vm)
        }

        /// aatrigon(...)：抗锯齿三角 = 兼容别名（同 trigon 组合）。
        #[pyfunction]
        fn aatrigon(f: PyObjectRef, args: TrigonArgs, vm: &VirtualMachine) -> PyResult<()> {
            trigon(f, args, vm)
        }

        /// filled_trigon(f, x1, y1, x2, y2, x3, y3)：填充三角。
        #[pyfunction]
        fn filled_trigon(f: PyObjectRef, args: TrigonArgs, vm: &VirtualMachine) -> PyResult<()> {
            filled_polygon3(&f, args.x1, args.y1, args.x2, args.y2, args.x3, args.y3, vm)
        }

        #[allow(clippy::too_many_arguments)]
        fn filled_polygon3(
            f: &PyObjectRef,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            x3: i32,
            y3: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let pts = SfPoints(vec![(x1, y1), (x2, y2), (x3, y3)]);
            match route(f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::polygon(&*s.surface, &pts.0, 0),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &pts.0, 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &pts.0, 0)
                })?,
            };
            Ok(())
        }

        #[allow(clippy::too_many_arguments)]
        fn polygon3(
            f: &PyObjectRef,
            x1: i32,
            y1: i32,
            x2: i32,
            y2: i32,
            x3: i32,
            y3: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                let pts = SfPoints(vec![(x1, y1), (x2, y2), (x3, y3)]);
            match route(f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::polygon(&*s.surface, &pts.0, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &pts.0, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &pts.0, 1)
                })?,
            };
            Ok(())
        }

        /// polygon(f, points)：1px 描边多边形（points = [(x, y), ...]）。
        #[pyfunction]
        fn polygon(f: PyObjectRef, points: SfPoints, vm: &VirtualMachine) -> PyResult<()> {
                match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::polygon(&*s.surface, &points.0, 1),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &points.0, 1)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &points.0, 1)
                })?,
            };
            Ok(())
        }

        /// aapolygon(f, points)：抗锯齿多边形 = 兼容别名。
        #[pyfunction]
        fn aapolygon(f: PyObjectRef, points: SfPoints, vm: &VirtualMachine) -> PyResult<()> {
            polygon(f, points, vm)
        }

        /// filled_polygon(f, points)：填充多边形。
        #[pyfunction]
        fn filled_polygon(
            f: PyObjectRef,
            points: SfPoints,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
                match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::polygon(&*s.surface, &points.0, 0),
                Target::Active => bridge::with_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &points.0, 0)
                })?,
                Target::Targets => bridge::with_mrt_session(vm, |t| {
                    starfish_pygame::draw::polygon(t, &points.0, 0)
                })?,
            };
            Ok(())
        }

        /// rectangle(f, rect)：1px 描边矩形（rect = (left, top, w, h)）。
        #[pyfunction]
        fn rectangle(f: PyObjectRef, rect: SfRect, vm: &VirtualMachine) -> PyResult<()> {
                let r = [rect.x, rect.y, rect.w, rect.h];
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::rect(&*s.surface, r, 1),
                Target::Active => bridge::with_session(vm, |t| starfish_pygame::draw::rect(t, r, 1))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| starfish_pygame::draw::rect(t, r, 1))?,
            };
            Ok(())
        }

        /// box(f, rect)：填充矩形。
        #[pyfunction(name = "box")]
        fn box_(f: PyObjectRef, rect: SfRect, vm: &VirtualMachine) -> PyResult<()> {
                let r = [rect.x, rect.y, rect.w, rect.h];
            match route(&f, vm)? {
                Target::Surf(s) => starfish_pygame::draw::rect(&*s.surface, r, 0),
                Target::Active => bridge::with_session(vm, |t| starfish_pygame::draw::rect(t, r, 0))?,
                Target::Targets => bridge::with_mrt_session(vm, |t| starfish_pygame::draw::rect(t, r, 0))?,
            };
            Ok(())
        }

        /// rounded_rectangle(f, rect, radius)：垫底——圆角语义
        /// v1 未建（glam 无 SDF 圆角管线），降级为直角描边矩形。
        #[pyfunction]
        fn rounded_rectangle(
            f: PyObjectRef,
            rect: SfRect,
            _radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            rectangle(f, rect, vm)
        }

        /// rounded_box(f, rect, radius)：垫底——降级为填充矩形。
        #[pyfunction]
        fn rounded_box(
            f: PyObjectRef,
            rect: SfRect,
            _radius: i32,
            vm: &VirtualMachine,
        ) -> PyResult<()> {
            box_(f, rect, vm)
        }
    }

    // ── mask：位面（碰撞检测/像素级遮挡；pygame.mask 语义）────────
    // 构建 = from_surface（影子快路径三端同步零回读；GPU 出生面原生
    // 走一次性回读，Web 侧同步面对 GPU 出生面不提供——from_surface_async

pub(crate) use gfxdraw::module_def;
