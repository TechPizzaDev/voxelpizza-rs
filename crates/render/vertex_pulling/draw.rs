use super::{cuboid_cache::CuboidBufferCache, index_buffer::GpuCuboidsIndexBuffer};
use bevy::{
    ecs::system::{lifetimeless::*, SystemParamItem},
    prelude::*,
    render::{
        render_asset::RenderAssets, render_phase::{
            PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
        }, render_resource::{BindGroup, IndexFormat}, sync_world::MainEntity, view::ViewUniformOffset
    },
};

pub(crate) type DrawCuboids = (
    SetItemPipeline,
    SetCuboidsViewBindGroup<0>,
    SetAuxBindGroup<1>,
    SetGpuTransformBufferBindGroup<2>,
    SetGpuCuboidBuffersBindGroup<3>,
    DrawVertexPulledCuboids,
);

#[derive(Default, Resource)]
pub struct ViewMeta {
    pub cuboids_view_bind_group: Option<BindGroup>,
}

pub(crate) struct SetCuboidsViewBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetCuboidsViewBindGroup<I> {
    type Param = SRes<ViewMeta>;
    type ItemQuery = ();
    type ViewQuery = Read<ViewUniformOffset>;

    #[inline]
    fn render<'w>(
        _item: &P,
        view_uniform_offset: &'_ ViewUniformOffset,
        _entity: Option<()>,
        view_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            view_meta
                .into_inner()
                .cuboids_view_bind_group
                .as_ref()
                .unwrap(),
            &[view_uniform_offset.offset],
        );
        RenderCommandResult::Success
    }
}

/// Holds the bind group for materials and clipping planes.
#[derive(Default, Resource)]
pub struct AuxiliaryMeta {
    pub bind_group: Option<BindGroup>,
}

pub(crate) struct SetAuxBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetAuxBindGroup<I> {
    type Param = (SRes<CuboidBufferCache>, SRes<AuxiliaryMeta>);
    type ItemQuery = ();
    type ViewQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _: Option<()>,
        (buffer_cache, aux_meta): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let buffer_cache = buffer_cache.into_inner();
        let aux_meta = aux_meta.into_inner();
        let entry = buffer_cache.entries.get(&item.main_entity()).unwrap();
        pass.set_bind_group(
            I,
            aux_meta.bind_group.as_ref().unwrap(),
            &[entry.material_index],
        );
        RenderCommandResult::Success
    }
}

#[derive(Default, Resource)]
pub struct TransformsMeta {
    pub transform_buffer_bind_group: Option<BindGroup>,
}

pub(crate) struct SetGpuTransformBufferBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetGpuTransformBufferBindGroup<I> {
    type Param = (SRes<CuboidBufferCache>, SRes<TransformsMeta>);
    type ItemQuery = ();
    type ViewQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _: Option<()>,
        (buffer_cache, transforms_meta): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let transforms_meta = transforms_meta.into_inner();
        let entry = buffer_cache.into_inner().entries.get(&item.main_entity()).unwrap();
        pass.set_bind_group(
            I,
            transforms_meta
                .transform_buffer_bind_group
                .as_ref()
                .unwrap(),
            &[entry.transform_index],
        );
        RenderCommandResult::Success
    }
}

pub(crate) struct SetGpuCuboidBuffersBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetGpuCuboidBuffersBindGroup<I> {
    type Param = SRes<CuboidBufferCache>;
    type ItemQuery = ();
    type ViewQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _: Option<()>,
        buffer_cache: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let entry = buffer_cache.into_inner().entries.get(&item.main_entity()).unwrap();
        pass.set_bind_group(I, entry.instance_buffer_bind_group.as_ref().unwrap(), &[]);
        RenderCommandResult::Success
    }
}

pub(crate) struct DrawVertexPulledCuboids;

impl<P: PhaseItem> RenderCommand<P> for DrawVertexPulledCuboids {
    type Param = (
        SRes<CuboidBufferCache>,
        SRes<RenderAssets<GpuCuboidsIndexBuffer>>,
    );
    type ItemQuery = ();
    type ViewQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _: Option<()>,
        (buffer_cache, index_buffers): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        use super::index_buffer::{CUBE_INDICES, CUBE_INDICES_HANDLE};
        let entry = buffer_cache.into_inner().entries.get(&item.main_entity()).unwrap();
        let num_cuboids = entry.instance_buffer.get().len().try_into().unwrap();
        let index_buffer = index_buffers
            .into_inner()
            .get(&CUBE_INDICES_HANDLE)
            .unwrap();
        pass.set_index_buffer(index_buffer.buffer.slice(..), 0, IndexFormat::Uint32);
        pass.draw_indexed(0..(CUBE_INDICES.len() as u32), 0, 0..num_cuboids);
        RenderCommandResult::Success
    }
}
