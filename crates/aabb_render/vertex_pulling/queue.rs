use crate::Cuboids;

use super::cuboid_cache::CuboidBufferCache;
use super::draw::DrawCuboids;
use super::pipeline::ViewCuboidsPipeline;

use bevy::core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey};
use bevy::ecs::component::Tick;
use bevy::prelude::*;
use bevy::render::mesh::allocator::SlabId;
use bevy::render::render_phase::{
    BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, ViewBinnedRenderPhases,
};
use bevy::render::view::{ExtractedView, RenderVisibleEntities};

pub(crate) fn queue_cuboids(
    opaque_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    buffer_cache: Res<CuboidBufferCache>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut views: Query<(
        &ExtractedView,
        &RenderVisibleEntities,
        &ViewCuboidsPipeline,
    )>,
    tick: Local<Tick>
) {
    let draw_cuboids = opaque_3d_draw_functions
        .read()
        .id::<DrawCuboids>();

    for (view, visible_entities, pipeline) in views.iter_mut() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        for &(render_entity, main_entity) in visible_entities.get::<Cuboids>().iter() {
            let Some(entry) = buffer_cache.entries.get(&main_entity) else {
                continue;
            };
            if !entry.enabled {
                continue;
            }

            opaque_phase.add(
                Opaque3dBatchSetKey {
                    pipeline: pipeline.0,
                    draw_function: draw_cuboids,
                    material_bind_group_index: None,
                    vertex_slab: SlabId::default(),
                    index_slab: None,
                    lightmap_slab: None,
                },
                Opaque3dBinKey {
                    asset_id: AssetId::<Mesh>::invalid().untyped(),
                },
                (render_entity, main_entity),
                InputUniformIndex::default(),
                BinnedRenderPhaseType::NonMesh,
                *tick
            );
        }
    }
}
