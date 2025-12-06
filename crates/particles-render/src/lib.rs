mod compute_billboard;

use bevy::prelude::*;
use bevy_catppuccin::CatppuccinTheme;
pub use compute_billboard::ComputeBillboardPlugin;

pub struct ParticlesRenderPlugin;

impl Plugin for ParticlesRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ComputeBillboardPlugin)
            .add_systems(Startup, setup_clear_color);
    }
}

fn setup_clear_color(mut commands: Commands, theme: Res<CatppuccinTheme>) {
    commands.insert_resource(ClearColor(theme.flavor.base));
}
