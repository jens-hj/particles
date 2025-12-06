use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

/// Detail level for the debug stats overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatsDetail {
    Minimal,   // FPS only
    #[default]
    Basic,     // FPS + Frame time
    Normal,    // + Entity count + Particle count
    Detailed,  // + Camera distance + Memory
}

/// Corner position for the stats overlay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Corner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Configuration for the debug stats overlay
#[derive(Resource, Debug, Clone, Copy)]
pub struct StatsConfig {
    pub detail: StatsDetail,
    pub corner: Corner,
    pub enabled: bool,
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self {
            detail: StatsDetail::Basic,
            corner: Corner::TopLeft,
            enabled: true,
        }
    }
}

pub struct DebugUiPlugin;

/// Timer to control how often stats are updated
#[derive(Resource)]
struct StatsUpdateTimer(Timer);

impl Default for StatsUpdateTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.25, TimerMode::Repeating))
    }
}

impl Plugin for DebugUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<StatsConfig>()
            .init_resource::<StatsUpdateTimer>()
            .add_systems(Startup, setup_stats_ui)
            .add_systems(Update, (handle_input, update_stats_text).chain());
    }
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<StatsConfig>,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            // Shift+F3: Cycle detail level
            config.detail = match config.detail {
                StatsDetail::Minimal => StatsDetail::Basic,
                StatsDetail::Basic => StatsDetail::Normal,
                StatsDetail::Normal => StatsDetail::Detailed,
                StatsDetail::Detailed => StatsDetail::Minimal,
            };
        } else {
            // F3: Toggle visibility
            config.enabled = !config.enabled;
        }
    }
}

#[derive(Component)]
struct StatsText;

fn setup_stats_ui(mut commands: Commands, config: Res<StatsConfig>) {
    let (left, right, top, bottom) = match config.corner {
        Corner::TopLeft => (Val::Px(10.0), Val::Auto, Val::Px(10.0), Val::Auto),
        Corner::TopRight => (Val::Auto, Val::Px(10.0), Val::Px(10.0), Val::Auto),
        Corner::BottomLeft => (Val::Px(10.0), Val::Auto, Val::Auto, Val::Px(10.0)),
        Corner::BottomRight => (Val::Auto, Val::Px(10.0), Val::Auto, Val::Px(10.0)),
    };

    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left,
            right,
            top,
            bottom,
            ..default()
        },
        StatsText,
    ));
}

fn update_stats_text(
    mut query: Query<(&mut Text, &mut Visibility), With<StatsText>>,
    config: Res<StatsConfig>,
    diagnostics: Res<DiagnosticsStore>,
    entities: Query<Entity>,
    time: Res<Time>,
    mut timer: ResMut<StatsUpdateTimer>,
) {
    if let Ok((mut text, mut visibility)) = query.single_mut() {
        if !config.enabled {
            *visibility = Visibility::Hidden;
            return;
        }
        *visibility = Visibility::Visible;

        // Only update text periodically to make it readable
        if !timer.0.tick(time.delta()).just_finished() {
            return;
        }

        let fps = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
            .unwrap_or(0.0);

        let stats = match config.detail {
            StatsDetail::Minimal => {
                format!("FPS:  {:>7.0}", fps)
            }
            StatsDetail::Basic => {
                let frame_time = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
                    .and_then(|ft| ft.smoothed())
                    .unwrap_or(0.0);
                format!("FPS:    {:>5.0}\nFrame:  {:>5.2}ms", fps, frame_time)
            }
            StatsDetail::Normal => {
                let frame_time = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
                    .and_then(|ft| ft.smoothed())
                    .unwrap_or(0.0);
                let entity_count = entities.iter().count();
                format!(
                    "FPS:      {:>7.0}\nFrame:    {:>5.2}ms\nEntities: {:>7}",
                    fps,
                    frame_time,
                    entity_count
                )
            }
            StatsDetail::Detailed => {
                let frame_time = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
                    .and_then(|ft| ft.smoothed())
                    .unwrap_or(0.0);
                let entity_count = entities.iter().count();
                format!(
                    "FPS:      {:>7.0}\nFrame:    {:>5.2}ms\nEntities: {:>7}\n\nF3: Toggle | Shift+F3: Cycle",
                    fps,
                    frame_time,
                    entity_count
                )
            }
        };

        **text = stats;
    }
}
