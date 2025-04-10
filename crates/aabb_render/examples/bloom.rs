use bevy::{
    core_pipeline::{
        bloom::Bloom,
        tonemapping::Tonemapping,
    },
    prelude::*,
};
use aabb_render::{Cuboid, CuboidMaterialId, Cuboids, VertexPullingRenderPlugin};
use smooth_bevy_cameras::{controllers::fps::*, LookTransformPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            watch_for_changes_override: Some(true),
            ..Default::default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            VertexPullingRenderPlugin { outlines: true },
            LookTransformPlugin,
            FpsCameraPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_fps_controller)
        .run();
}

fn setup(mut commands: Commands) {
    let colors = [
        Srgba::RED,
        Srgba::GREEN,
        Srgba::BLUE,
        Srgba::rgb(1.0, 1.0, 0.0),
        Srgba::rgb(1.0, 0.0, 1.0),
    ];

    let mut cuboids = Vec::new();
    for x in 0..10 {
        for y in 0..10 {
            let min = Vec3::new(x as f32 - 5.0, 0.0, y as f32 - 5.0);
            let max = min + Vec3::ONE;
            let color = colors[(x + y) % colors.len()].to_u8_array();
            let mut cuboid = Cuboid::new(min, max, u32::from_le_bytes(color));
            if min.length() < 3.0 {
                cuboid.make_emissive();
            }
            cuboids.push(cuboid);
        }
    }

    let cuboids = Cuboids::new(cuboids);
    let aabb = cuboids.aabb();
    commands.spawn((cuboids, aabb, CuboidMaterialId(0)));

    commands
        .spawn((
            Camera {
                hdr: true,
                ..default()
            },
            Camera3d::default(),
            Tonemapping::TonyMcMapface,
            Bloom {
                intensity: 0.2,
                low_frequency_boost: 0.8,
                low_frequency_boost_curvature: 0.7,
                ..default()
            },
        ))
        .insert(FpsCameraBundle::new(
            FpsCameraController {
                translate_sensitivity: 10.0,
                enabled: false,
                ..default()
            },
            Vec3::splat(10.0),
            Vec3::ZERO,
            Vec3::Y,
        ));
}

fn toggle_fps_controller(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut controller: Query<&mut FpsCameraController>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        controller.single_mut().enabled = true;
    }
}
