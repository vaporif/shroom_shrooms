use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// Packed uniform struct — matches the WGSL `VignetteUniforms` struct exactly.
///
/// Layout: color (16 bytes) + intensity (4 bytes) + _padding (12 bytes) = 32 bytes.
#[derive(ShaderType, Debug, Clone)]
pub struct VignetteUniforms {
    pub color: LinearRgba,
    pub intensity: f32,
    // Pad to 16-byte boundary so the struct size is a multiple of the largest alignment.
    pub _padding: Vec3,
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct VignetteMaterial {
    #[uniform(0)]
    pub uniforms: VignetteUniforms,
}

impl Material2d for VignetteMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/vignette.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Marker component for the fullscreen vignette quad.
#[derive(Component)]
pub struct VignetteOverlay;

/// Startup system: spawn a large quad with `VignetteMaterial` at a high Z so it
/// composites over everything else.
pub fn spawn_vignette(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<VignetteMaterial>>,
) {
    commands.spawn((
        VignetteOverlay,
        Mesh2d(meshes.add(Rectangle::new(4000.0, 4000.0))),
        MeshMaterial2d(materials.add(VignetteMaterial {
            uniforms: VignetteUniforms {
                color: LinearRgba::new(0.0, 0.0, 0.0, 1.0),
                intensity: 0.6,
                _padding: Vec3::ZERO,
            },
        })),
        Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
    ));
}

/// Keep the vignette quad centered on the camera so it covers the viewport.
pub fn update_vignette(
    camera_q: Query<&Transform, With<Camera2d>>,
    mut vignette_q: Query<&mut Transform, (With<VignetteOverlay>, Without<Camera2d>)>,
) {
    let Ok(cam_tf) = camera_q.single() else {
        return;
    };
    for mut vt in vignette_q.iter_mut() {
        vt.translation.x = cam_tf.translation.x;
        vt.translation.y = cam_tf.translation.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vignette_material_has_intensity() {
        let mat = VignetteMaterial {
            uniforms: VignetteUniforms {
                color: LinearRgba::new(0.0, 0.0, 0.0, 1.0),
                intensity: 0.6,
                _padding: Vec3::ZERO,
            },
        };
        assert_eq!(mat.uniforms.intensity, 0.6);
    }
}
