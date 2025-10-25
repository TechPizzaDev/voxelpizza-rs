use super::buffers::*;
use super::cuboid_cache::CuboidBufferCache;
use super::draw::{AuxiliaryMeta, DrawCuboids, TransformsMeta, ViewMeta};
use super::extract::{extract_clipping_planes, extract_cuboids};
use super::index_buffer::*;
use super::pipeline::*;
use super::prepare::*;
use super::queue::queue_cuboids;

use crate::CuboidMaterialMap;

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetPlugin;
use bevy::render::render_phase::AddRenderCommand;
use bevy::render::render_resource::SpecializedRenderPipelines;
use bevy::render::view::prepare_view_uniforms;
use bevy::render::{Render, RenderApp, RenderSet};

pub(crate) fn cuboid_shader_path() -> AssetPath<'static> {
    AssetPath::from_path(&embedded_path!("vertex_pulling/", "vertex_pulling.wgsl"))
        .with_source("embedded")
        .into_owned()
}

/// Renders the [`Cuboids`](crate::Cuboids) component using the "vertex pulling" technique.
#[derive(Default)]
pub struct VertexPullingRenderPlugin {
    pub outlines: bool,
}

impl Plugin for VertexPullingRenderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "vertex_pulling/", "vertex_pulling.wgsl");

        app.init_resource::<CuboidMaterialMap>();

        app.init_asset::<CuboidsIndexBuffer>()
            .add_plugins(RenderAssetPlugin::<GpuCuboidsIndexBuffer>::default());

        app.world_mut()
            .resource_mut::<Assets<CuboidsIndexBuffer>>()
            .insert(CUBE_INDICES_HANDLE.id(), CuboidsIndexBuffer);
    }

    fn finish(&self, app: &mut App) {
        let world = app.world_mut();
        let maybe_msaa = world.query::<&Msaa>().single(world).ok().cloned();
        let render_app = app.sub_app_mut(RenderApp);

        if let Some(msaa) = maybe_msaa {
            render_app.world_mut().spawn(msaa.clone());
        }
        let mut shader_defs = CuboidsShaderDefs::default();
        if self.outlines {
            shader_defs.enable_outlines();
        }
        render_app.insert_resource(shader_defs);

        render_app
            .add_render_command::<Opaque3d, DrawCuboids>()
            .init_resource::<SpecializedRenderPipelines<CuboidsPipeline>>()
            .init_resource::<AuxiliaryMeta>()
            .init_resource::<CuboidBufferCache>()
            .init_resource::<DynamicUniformBufferOfCuboidMaterial>()
            .init_resource::<DynamicUniformBufferOfCuboidTransforms>()
            .init_resource::<TransformsMeta>()
            .init_resource::<UniformBufferOfGpuClippingPlaneRanges>()
            .init_resource::<ViewMeta>()
            .init_resource::<CuboidsBindGroupLayouts>()
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
                    prepare_cuboids_pipelines,
                )
                    .in_set(RenderSet::Prepare),
            )
            .add_systems(Render, queue_cuboids.in_set(RenderSet::Queue));
    }
}
