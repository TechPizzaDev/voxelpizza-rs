use super::buffers::*;
use super::cuboid_cache::CuboidBufferCache;
use super::draw::{AuxiliaryMeta, DrawCuboids, TransformsMeta, ViewMeta};
use super::extract::{extract_clipping_planes, extract_cuboids};
use super::pipeline::{CuboidsPipelines, CuboidsShaderDefs};
use super::prepare::{
    prepare_auxiliary_bind_group, prepare_clipping_planes, prepare_cuboid_transforms,
    prepare_cuboids, prepare_cuboids_view_bind_group, prepare_materials,
};
use super::queue::queue_cuboids;
use crate::vertex_pulling::index_buffer::GpuCuboidsIndexBuffer;
use crate::{CuboidMaterialMap, Cuboids};
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::prelude::*;
use bevy::render::view::{check_visibility, prepare_view_uniforms, VisibilitySystems};
use bevy::render::{render_phase::AddRenderCommand, RenderApp};
use bevy::render::{Render, RenderSet};

/// Renders the [`Cuboids`](crate::Cuboids) component using the "vertex pulling" technique.
#[derive(Default)]
pub struct VertexPullingRenderPlugin {
    pub outlines: bool,
}

impl Plugin for VertexPullingRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CuboidMaterialMap>();
        
        use super::index_buffer::{CuboidsIndexBuffer, CUBE_INDICES_HANDLE};
        use bevy::render::render_asset::RenderAssetPlugin;
        app.init_asset::<CuboidsIndexBuffer>()
            .add_plugins(RenderAssetPlugin::<GpuCuboidsIndexBuffer>::default());
        app.world_mut()
            .resource_mut::<Assets<CuboidsIndexBuffer>>()
            .insert(CUBE_INDICES_HANDLE.id(), CuboidsIndexBuffer);

        app.add_systems(
            PostUpdate,
            check_visibility::<With<Cuboids>>.in_set(VisibilitySystems::CheckVisibility),
        );
    }

    fn finish(&self, app: &mut App) {
        let maybe_msaa = app.world().get_resource::<Msaa>().cloned();
        let render_app = app.sub_app_mut(RenderApp);

        if let Some(msaa) = maybe_msaa {
            render_app.insert_resource(msaa);
        }
        let mut shader_defs = CuboidsShaderDefs::default();
        if self.outlines {
            shader_defs.enable_outlines();
        }
        render_app.insert_resource(shader_defs);

        render_app
            .add_render_command::<Opaque3d, DrawCuboids>()
            .init_resource::<AuxiliaryMeta>()
            .init_resource::<CuboidBufferCache>()
            .init_resource::<CuboidsPipelines>()
            .init_resource::<DynamicUniformBufferOfCuboidMaterial>()
            .init_resource::<DynamicUniformBufferOfCuboidTransforms>()
            .init_resource::<TransformsMeta>()
            .init_resource::<UniformBufferOfGpuClippingPlaneRanges>()
            .init_resource::<ViewMeta>()
            .add_systems(ExtractSchedule, (extract_cuboids, extract_clipping_planes))
            .add_systems(
                Render,
                (
                    prepare_materials,
                    prepare_clipping_planes,
                    prepare_auxiliary_bind_group
                        .after(prepare_materials)
                        .after(prepare_clipping_planes),
                    prepare_cuboid_transforms,
                    prepare_cuboids,
                    prepare_cuboids_view_bind_group.after(prepare_view_uniforms),
                )
                    .in_set(RenderSet::Prepare),
            )
            .add_systems(Render, queue_cuboids.in_set(RenderSet::Queue));
    }
}
