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
/// flagship; gravity is applied at step time).
pub(crate) fn make_pipeline(particle_radius: SReal, smoothing_factor: SReal) -> FluidsPipeline {
    FluidsPipeline::new(particle_radius, smoothing_factor)
}

/// Fluid constructor shim (positions in, velocities zeroed - salva
/// Fluid::new fills them).
pub(crate) fn make_fluid(positions: Vec<SVector>, particle_radius: SReal) -> Fluid {
    Fluid::new(positions, particle_radius, 1000.0, InteractionGroups::all())
}

/// Boundary constructor shim (static zero-velocity wall particles).
pub(crate) fn make_boundary(positions: Vec<SVector>) -> Boundary {
    Boundary::new(positions, InteractionGroups::all())
}

pub(crate) fn make_solver() -> DFSPHSolver {
    DFSPHSolver::new()
}
