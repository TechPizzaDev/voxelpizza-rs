
use bevy::{
    core_pipeline::{
        bloom::Bloom,
        tonemapping::Tonemapping,
    },
    prelude::*,
};
use smooth_bevy_cameras::{controllers::fps::*, LookTransformPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            watch_for_changes_override: Some(true),
            ..Default::default()
        }))
        .add_plugins((LookTransformPlugin, FpsCameraPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_fps_controller)
        .run();
}

fn setup(mut commands: Commands) {
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
                ..Default::default()
            },
        ))
        .insert(FpsCameraBundle::new(
            FpsCameraController {
                translate_sensitivity: 10.0,
                enabled: false,
                ..Default::default()
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
