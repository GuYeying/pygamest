//! fluid 2D runtime helpers (pure Rust - no python bindings here; the
//! pyclass facade lives in pygame_physics/mod.rs). salva2d 0.11 = the
//! same src tree as salva3d compiled under cfg(dim2) - API identical,
//! only mechanical differences: Vector2, 2-arg RapierVector::new,
//! 2-loop rectangle sampling, dim2 rapier prelude.
//!
//! FluidsPipeline = salva LiquidWorld + rapier ColliderCouplingSet.
//! Coupling step takes `&ColliderSet` + `&mut RigidBodySet` straight from
//! the solid world (fields are pub on rapier PhysicsWorld).

pub(crate) use salva2d::integrations::rapier::FluidsPipeline;
pub(crate) type SVector = nalgebra035::Vector2<f32>;
use salva2d::object::interaction_groups::InteractionGroups;
pub(crate) use salva2d::object::{Boundary, Fluid};
use salva2d::solver::DFSPHSolver;

pub(crate) type SReal = f32;

/// rectangle uniform grid sampling (spacing defaults to 2x particle
/// radius, the SPH init convention: neighbor centers <= 2x radius).
pub(crate) fn sample_cuboid_grid(
    corner: [SReal; 2],
    dims: [SReal; 2],
    spacing: SReal,
) -> Vec<SVector> {
    let nx = ((dims[0] / spacing).floor() as i32).max(1);
    let ny = ((dims[1] / spacing).floor() as i32).max(1);
    let mut out = Vec::with_capacity((nx * ny) as usize);
    for ix in 0..nx {
        for iy in 0..ny {
            out.push(SVector::new(
                corner[0] + ix as SReal * spacing,
                corner[1] + iy as SReal * spacing,
            ));
        }
    }
    out
}

/// container boundary sampling (four walls, static zero-velocity
/// particles - the 2D counterpart of the 3D six-face shell).
pub(crate) fn sample_shell(corner: [SReal; 2], dims: [SReal; 2], spacing: SReal) -> Vec<SVector> {
    let mut pts = Vec::new();
    let nx = ((dims[0] / spacing).floor() as i32).max(1);
    let ny = ((dims[1] / spacing).floor() as i32).max(1);
    // bottom / top walls (two horizontal slices)
    for ix in 0..=nx {
        for side in 0..2 {
            let yo = if side == 0 { 0.0 } else { dims[1] };
            pts.push(SVector::new(
                corner[0] + ix as SReal * spacing,
                corner[1] + yo,
            ));
        }
    }
    // left / right walls (two vertical slices)
    for iy in 0..=ny {
        for side in 0..2 {
            let xo = if side == 0 { 0.0 } else { dims[0] };
            pts.push(SVector::new(
                corner[0] + xo,
                corner[1] + iy as SReal * spacing,
            ));
        }
    }
    pts
}

/// Build the pipeline (DFSPH pressure solver - salva's volume-conserving
/// flagship; gravity is applied at step time). Stable with the DC-coupling
/// boundary (rapier contact projection - the only reliable boundary for
/// impact/sloshing scenes in salva 0.11, see sync_coupling notes).
pub(crate) fn make_pipeline(particle_radius: SReal, smoothing_factor: SReal) -> FluidsPipeline {
    FluidsPipeline::new(particle_radius, smoothing_factor)
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
        .push(Box::new(salva2d::solver::XSPHViscosity::new(2.0, 1.0)));
    fluid
}

/// Boundary constructor shim (static zero-velocity wall particles).
pub(crate) fn make_boundary(positions: Vec<SVector>) -> Boundary {
    Boundary::new(positions, InteractionGroups::all())
}

pub(crate) fn make_solver() -> DFSPHSolver {
    DFSPHSolver::new()
}

// ── rapier coupling (static/dynamic rigid body <-> fluid, 2D) ──
// Requires physics2d (same rapier2d 0.36.1 as salva2d's own dependency -
// single lock version, so ColliderSet/RigidBodySet types are identical).

#[cfg(feature = "physics2d")]
pub(crate) mod coupling {
    use super::{make_boundary, FluidsPipeline, SReal};
    use rapier_starfish_debug::dim2::prelude::{
        ColliderSet, RigidBodySet, Vector as RapierVector,
    };

    /// Register every not-yet-coupled collider as a salva boundary
    /// (incremental - existing coupling entries are kept as-is). Sampling =
    /// DynamicContactSampling: salva projects penetrating particles out of
    /// the collider shape and kills their inward normal velocity each step
    /// (hard contact response - no sample density needed). StaticSampling
    /// (boundary point clouds) blows the pressure solve in impact scenes -
    /// the 3D sibling documented the full experiment chain (2026-11-18).
    /// Works for any collider shape, rotated colliders included. Force
    /// feedback is auto-disabled for non-dynamic bodies by salva itself.
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
            // Coupling contract: static (fixed) bodies only. salva's DC
            // boundary force feedback auto-arms for dynamic parents and
            // its numerics are unstable (a falling cube through the fluid
            // collects the full projected-particle pressure as impulses ->
            // energy explosion, fly-off observed 2026-11-18) - dynamic
            // coupling is v2 work, skip registration here.
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
                salva2d::integrations::rapier::ColliderSampling::DynamicContactSampling,
            );
        }
    }

    /// Coupled step shim: FluidsPipeline::step(gravity, dt, colliders,
    /// bodies) - the rapier-integration entry that runs step_with_coupling
    /// underneath (registered boundaries get their positions re-sampled
    /// from collider transforms every step).
    pub(crate) fn step_coupled(
        pipeline: &mut FluidsPipeline,
        gravity: [SReal; 2],
        dt: SReal,
        colliders: &ColliderSet,
        bodies: &mut RigidBodySet,
    ) {
        let g = RapierVector::new(gravity[0], gravity[1]);
        pipeline.step(&g, dt, colliders, bodies);
    }
}
