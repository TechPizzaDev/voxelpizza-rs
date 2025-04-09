use crate::Cuboids;

use super::cuboid_cache::CuboidBufferCache;
use super::draw::DrawCuboids;
use super::pipeline::ViewCuboidsPipeline;

use bevy::core_pipeline::core_3d::{Opaque3d, Opaque3dBinKey};
use bevy::prelude::*;
use bevy::render::render_phase::{BinnedRenderPhaseType, DrawFunctions, ViewBinnedRenderPhases};
use bevy::render::view::RenderVisibleEntities;

pub(crate) fn queue_cuboids(
    opaque_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    buffer_cache: Res<CuboidBufferCache>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut views: Query<(Entity, &RenderVisibleEntities, &ViewCuboidsPipeline)>,
) {
    let draw_cuboids = opaque_3d_draw_functions
        .read()
        .get_id::<DrawCuboids>()
        .unwrap();

    for (view_entity, visible_entities, pipeline) in views.iter_mut() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view_entity) else {
            continue;
        };

        for &(render_entity, main_entity) in visible_entities.iter::<With<Cuboids>>() {
            let Some(entry) = buffer_cache.entries.get(&main_entity) else {
                continue;
            };
            if !entry.enabled {
                continue;
            }

            opaque_phase.add(
                Opaque3dBinKey {
                    pipeline: pipeline.0,
                    draw_function: draw_cuboids,
                    asset_id: AssetId::<Mesh>::invalid().untyped(),
                    material_bind_group_id: None,
                    lightmap_image: None,
                },
                (render_entity, main_entity),
                BinnedRenderPhaseType::NonMesh,
            );
        }
    }
}
