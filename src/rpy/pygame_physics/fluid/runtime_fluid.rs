//! fluid runtime helpers (pure Rust - no python bindings here; the
//! pyclass facade lives in pygame_physics/mod.rs, same layout as solid).
//!
//! FluidsPipeline = salva LiquidWorld + rapier ColliderCouplingSet.
//! Coupling step takes `&ColliderSet` + `&mut RigidBodySet` straight from
//! the solid world (fields are pub on rapier PhysicsWorld).

pub(crate) use salva3d::integrations::rapier::FluidsPipeline;
pub(crate) type SVector = nalgebra035::Vector3<f32>;
use salva3d::object::interaction_groups::InteractionGroups;
pub(crate) use salva3d::object::{Boundary, Fluid};
use salva3d::solver::DFSPHSolver;

pub(crate) type SReal = f32;

/// cuboid uniform grid sampling (spacing defaults to 2x particle radius,
/// the SPH init convention: neighbor centers <= 2x radius for kernel
/// coverage).
pub(crate) fn sample_cuboid_grid(
    corner: [SReal; 3],
    dims: [SReal; 3],
    spacing: SReal,
) -> Vec<SVector> {
    let nx = ((dims[0] / spacing).floor() as i32).max(1);
    let ny = ((dims[1] / spacing).floor() as i32).max(1);
    let nz = ((dims[2] / spacing).floor() as i32).max(1);
    let mut out = Vec::with_capacity((nx * ny * nz) as usize);
    for ix in 0..nx {
        for iy in 0..ny {
            for iz in 0..nz {
                out.push(SVector::new(
                    corner[0] + ix as SReal * spacing,
                    corner[1] + iy as SReal * spacing,
                    corner[2] + iz as SReal * spacing,
                ));
            }
        }
    }
    out
}

/// container shell sampling (six faces, static zero-velocity particles).
pub(crate) fn sample_shell(corner: [SReal; 3], dims: [SReal; 3], spacing: SReal) -> Vec<SVector> {
    let mut pts = Vec::new();
    let nx = ((dims[0] / spacing).floor() as i32).max(1);
    let ny = ((dims[1] / spacing).floor() as i32).max(1);
    let nz = ((dims[2] / spacing).floor() as i32).max(1);
    // bottom / top faces (xz plane, two slices)
    for ix in 0..=nx {
        for iz in 0..=nz {
            for side in 0..2 {
                let yo = if side == 0 { 0.0 } else { dims[1] };
                pts.push(SVector::new(
                    corner[0] + ix as SReal * spacing,
                    corner[1] + yo,
                    corner[2] + iz as SReal * spacing,
                ));
            }
        }
    }
    // side walls (yz planes, two slices)
    for iy in 0..=ny {
        for iz in 0..=nz {
            for side in 0..2 {
                let xo = if side == 0 { 0.0 } else { dims[0] };
                pts.push(SVector::new(
                    corner[0] + xo,
                    corner[1] + iy as SReal * spacing,
                    corner[2] + iz as SReal * spacing,
                ));
            }
        }
    }
    pts
}

/// Build the pipeline (DFSPH pressure solver - salva's volume-conserving
/// flagship; gravity is applied at step time). Stable with the DC-coupling
/// boundary (rapier contact projection - the only reliable boundary for
/// impact/sloshing scenes in salva 0.11, see sync_coupling notes).
pub(crate) fn make_pipeline(particle_radius: SReal, smoothing_factor: SReal) -> FluidsPipeline {
    let mut pipeline = FluidsPipeline::new(particle_radius, smoothing_factor);
    // boundary_force_coefficient = 0（**单向耦合定约**，2026-11-19，
    // 2D 同款）：salva 的 rapier 集成每步对 dynamic 父体强制重开力反馈
    //（手动置 boundary.forces = None 会被 update_boundaries 覆盖）——
    // 冲量直灌刚体 = 落水方块能量爆炸/飞出（dynamic 耦合因此被整体
    // 跳过）。系数归零在 transmit 端消灭冲量（fixed 边界 forces 恒
    // None 无影响），DC 位置投影照常 = 粒子被动态刚体推开而刚体不受
    // 力；双向（浮力/拖拽）= 后续以非零系数+稳定性实验立项
    pipeline.liquid_world.boundary_force_coefficient = 0.0;
    pipeline
}

/// DFSPH pipeline (volume-conserving flagship - kept for comparison / v2
/// parameter tuning; unstable on boundary contact at this writing).
pub(crate) fn make_iisph_pipeline(
    particle_radius: SReal,
    smoothing_factor: SReal,
) -> FluidsPipeline {
    FluidsPipeline::new_with_solver(
        salva3d::solver::IISPHSolver::<salva3d::kernel::CubicSplineKernel, salva3d::kernel::CubicSplineKernel>::new(),
        particle_radius,
        smoothing_factor,
    )
}

/// Fluid constructor shim (positions in, velocities zeroed - salva
/// Fluid::new fills them). XSPH viscosity attached by default: fluid
/// particle velocities converge toward the neighbor average each step,
/// which reads as thick honey-like clumping flow (plain SPH reads as
/// ballistic droplets - user feedback 2026-11-18). Coefficients: fluid
/// 2.0 (strong smoothing = viscous look), boundary 1.0 (damped wall
/// shear). Above ~3 the velocity smoothing starts fighting the DFSPH
/// pressure solve - keep under that.
pub(crate) fn make_fluid(positions: Vec<SVector>, particle_radius: SReal) -> Fluid {
    let mut fluid = Fluid::new(positions, particle_radius, 1000.0, InteractionGroups::all());
    fluid
        .nonpressure_forces
        .push(Box::new(salva3d::solver::XSPHViscosity::new(2.0, 1.0)));
    fluid
}

/// Boundary constructor shim (static zero-velocity wall particles).
pub(crate) fn make_boundary(positions: Vec<SVector>) -> Boundary {
    Boundary::new(positions, InteractionGroups::all())
}

pub(crate) fn make_solver() -> DFSPHSolver {
    DFSPHSolver::new()
}

// ── rapier coupling (batch 3: static/dynamic rigid body <-> fluid) ──
// Requires physics3d (same rapier3d 0.36.1 as salva's own dependency -
// single lock version, so ColliderSet/RigidBodySet types are identical).

#[cfg(feature = "physics3d")]
pub(crate) mod coupling {
    use super::{make_boundary, sample_shell, FluidsPipeline, SReal};
    use rapier_starfish_debug::dim3::prelude::{
        ColliderSet, RigidBodySet, Vector as RapierVector,
    };

    /// Register every not-yet-coupled collider as a salva boundary
    /// (incremental - existing coupling entries are kept as-is). Sampling =
    /// DynamicContactSampling: salva projects penetrating particles out of
    /// the collider shape and kills their inward normal velocity each step
    /// (hard contact response - no sample density needed). StaticSampling
    /// (shell point clouds) was tried first and abandoned: overlapping
    /// boundary clouds where two colliders meet (ramp foot on floor) double
    /// the density contribution and blow the DFSPH pressure solve apart.
    /// Works for any collider shape, rotated colliders included (position
    /// transform happens inside salva's update_boundaries). Force feedback
    /// is auto-disabled for non-dynamic bodies by salva itself
    /// (boundary.forces = None when the parent body is not dynamic).
    ///
    /// 耦合定约（2026-11-23 复测回封 8957dc3 语义）：**仅 fixed 刚体参与**。
    /// dynamic 照常注册曾于 bb46a1b 开放（boundary_force_coefficient=0
    /// 单向），fluid3d_game 三球对照复测推翻：被动态体穿过的水块全体
    /// 失控穿底自由落体（逐句柄实锤 handle#0 y=-11.25，未接触句柄完好；
    /// 大球 0.2>粒子半径 0.09 + 高位落水复测同炸）——DC 投影对运动边界
    /// 接触粒子注入失控速度，一步穿 0.12m 薄底且投影无法回收。dynamic
    /// ×流体耦合整体挂账 v2（位置投影+力回传一并重立项）。
    pub(crate) fn sync_coupling(
        pipeline: &mut FluidsPipeline,
        colliders: &ColliderSet,
        bodies: &RigidBodySet,
        _particle_radius: SReal,
    ) {
        for (handle, collider) in colliders.iter() {
            if pipeline.coupling.entries.contains_key(&handle) {
                continue;
            }
            let dynamic = collider
                .parent()
                .and_then(|p| bodies.get(p))
                .map(|b| b.is_dynamic())
                .unwrap_or(false);
            if dynamic {
                continue;
            }
            let bh = pipeline.liquid_world.add_boundary(make_boundary(Vec::new()));
            pipeline.coupling.register_coupling(
                bh,
                handle,
                salva3d::integrations::rapier::ColliderSampling::DynamicContactSampling,
            );
        }
    }

    /// Coupled step shim: FluidsPipeline::step(gravity, dt, colliders,
    /// bodies) - the rapier-integration entry that runs step_with_coupling
    /// underneath (registered boundaries get their positions re-sampled
    /// from collider transforms every step).
    pub(crate) fn step_coupled(
        pipeline: &mut FluidsPipeline,
        gravity: [SReal; 3],
        dt: SReal,
        colliders: &ColliderSet,
        bodies: &mut RigidBodySet,
    ) {
        let g = RapierVector::new(gravity[0], gravity[1], gravity[2]);
        pipeline.step(&g, dt, colliders, bodies);
    }
}
