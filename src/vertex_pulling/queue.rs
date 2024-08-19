use crate::Cuboids;

use super::cuboid_cache::CuboidBufferCache;
use super::draw::DrawCuboids;
use super::pipeline::CuboidsPipelines;

use bevy::core_pipeline::core_3d::{Opaque3d, Opaque3dBinKey};
use bevy::prelude::*;
use bevy::render::render_phase::{BinnedRenderPhaseType, DrawFunctions, ViewBinnedRenderPhases};
use bevy::render::view::{ExtractedView, VisibleEntities};

pub(crate) fn queue_cuboids(
    cuboids_pipelines: Res<CuboidsPipelines>,
    opaque_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    buffer_cache: Res<CuboidBufferCache>,
    mut views: Query<(Entity, &ExtractedView, &VisibleEntities)>,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
) {
    let draw_cuboids = opaque_3d_draw_functions
        .read()
        .get_id::<DrawCuboids>()
        .unwrap();

    for (view_entity, view, visible_entities) in views.iter_mut() {
        let Some(opaque_phase) = opaque_render_phases.get_mut(&view_entity) else {
            continue;
        };

        for &entity in visible_entities.iter::<With<Cuboids>>(){
            let Some(entry) = buffer_cache.entries.get(&entity) else { 
                continue 
            };
            if !entry.enabled {
                continue;
            }
            
            let pipeline = if view.hdr {
                cuboids_pipelines.hdr_pipeline_id
            } else {
                cuboids_pipelines.pipeline_id
            };
            opaque_phase.add(
                Opaque3dBinKey {
                    pipeline,
                    draw_function: draw_cuboids,
                    asset_id: AssetId::<Mesh>::invalid().untyped(),
                    material_bind_group_id: None,
                    lightmap_image: None,
                },
                entity,
                BinnedRenderPhaseType::NonMesh,
            );
        }
    }
}
