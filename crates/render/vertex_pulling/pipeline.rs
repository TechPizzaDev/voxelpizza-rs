use crate::CUBOID_SHADER_HANDLE;
use crate::clipping_planes::GpuClippingPlaneRanges;
use crate::{CuboidMaterial, cuboids::CuboidsTransform};

use bevy::render::render_resource::ShaderDefVal;
use bevy::render::view::{ExtractedView, ViewTarget};
use bevy::{
    prelude::*,
    render::{
        mesh::PrimitiveTopology, render_resource::*, renderer::RenderDevice, view::ViewUniform,
    },
};

#[derive(Component)]
pub struct ViewCuboidsPipeline(pub CachedRenderPipelineId);

/// A key that uniquely identifies depth of field pipelines.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CuboidsPipelineKey {
    /// Whether we're using HDR.
    pub hdr: bool,
    /// Whether the render target is multisampled.
    pub sample_count: u32,
}

#[derive(Resource)]
pub(crate) struct CuboidsPipeline {
    layouts: CuboidsBindGroupLayouts,
    defs: CuboidsShaderDefs,
}

#[derive(Resource, Clone)]
pub(crate) struct CuboidsBindGroupLayouts {
    pub aux_layout: BindGroupLayout,
    pub cuboids_layout: BindGroupLayout,
    pub transforms_layout: BindGroupLayout,
    pub view_layout: BindGroupLayout,
}

impl FromWorld for CuboidsBindGroupLayouts {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let view_layout = render_device.create_bind_group_layout(
            Some("cuboids_view_layout"),
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(ViewUniform::min_size().get()),
                },
                count: None,
            }],
        );

        let aux_layout = render_device.create_bind_group_layout(
            Some("aux_layout"),
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(CuboidMaterial::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuClippingPlaneRanges::min_size()),
                    },
                    count: None,
                },
            ],
        );

        let transforms_layout = render_device.create_bind_group_layout(
            Some("transforms_layout"),
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    // We always have a single transform for each instance buffer.
                    min_binding_size: Some(CuboidsTransform::min_size()),
                },
                count: None,
            }],
        );

        let cuboids_layout = render_device.create_bind_group_layout(
            Some("cuboid_instances_layout"),
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(0),
                },
                count: None,
            }],
        );

        Self {
            view_layout,
            aux_layout,
            cuboids_layout,
            transforms_layout,
        }
    }
}

impl SpecializedRenderPipeline for CuboidsPipeline {
    type Key = CuboidsPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let layout = vec![
            self.layouts.view_layout.clone(),
            self.layouts.aux_layout.clone(),
            self.layouts.transforms_layout.clone(),
            self.layouts.cuboids_layout.clone(),
        ];

        let vertex = VertexState {
            shader: CUBOID_SHADER_HANDLE,
            shader_defs: self.defs.vertex.clone(),
            entry_point: "vertex".into(),
            buffers: vec![],
        };
        let fragment = FragmentState {
            shader: CUBOID_SHADER_HANDLE,
            shader_defs: self.defs.fragment.clone(),
            entry_point: "fragment".into(),
            targets: vec![Some(ColorTargetState {
                format: if key.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
                blend: Some(BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
        };

        let primitive = PrimitiveState {
            front_face: FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: PolygonMode::Fill,
            conservative: false,
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
        };
        let depth_stencil = Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Greater,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
        });
        let multisample = MultisampleState {
            count: key.sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        RenderPipelineDescriptor {
            label: Some("cuboids_pipeline".into()),
            layout,
            vertex,
            fragment: Some(fragment),
            primitive,
            depth_stencil,
            multisample,
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        }
    }
}

pub fn prepare_cuboids_pipelines(
    mut commands: Commands,
    layouts: Res<CuboidsBindGroupLayouts>,
    shader_defs: Res<CuboidsShaderDefs>,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<CuboidsPipeline>>,
    msaa_targets: Query<(Entity, &ExtractedView, &Msaa)>,
) {
    for (view_entity, view, msaa) in msaa_targets.iter() {
        let pipeline = CuboidsPipeline {
            layouts: layouts.clone(),
            defs: shader_defs.clone(),
        };

        let (hdr, sample_count) = (view.hdr, msaa.samples());

        commands
            .entity(view_entity)
            .insert(ViewCuboidsPipeline(pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                CuboidsPipelineKey { hdr, sample_count },
            )));
    }
}

#[derive(Clone, Default, Resource)]
pub(crate) struct CuboidsShaderDefs {
    pub vertex: Vec<ShaderDefVal>,
    pub fragment: Vec<ShaderDefVal>,
}

impl CuboidsShaderDefs {
    pub fn enable_outlines(&mut self) {
        self.vertex.push("OUTLINES".into());
        self.fragment.push("OUTLINES".into());
    }
}
