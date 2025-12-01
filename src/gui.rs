use bevy::{
    app::Plugin,
    camera::ClearColor,
    color::Color,
    ecs::{
        schedule::{IntoScheduleConfigs, SystemCondition},
        system::{Local, Res, ResMut},
    },
    input::keyboard::KeyCode,
    state::{
        condition::in_state,
        state::{NextState, State},
    },
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::{resource::LoadingProgress, state::GameState};

pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.15)))
            .add_plugins(EguiPlugin::default())
            .add_plugins(
                WorldInspectorPlugin::default().run_if(
                    bevy::input::common_conditions::input_toggle_active(true, KeyCode::Escape)
                        .and(in_state(GameState::Playing)),
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                display_loading_screen.run_if(
                    in_state(GameState::Loading)
                        .or(in_state(GameState::PostLoading).or(in_state(GameState::PreLoading))),
                ),
            );
    }
}

fn display_loading_screen(
    mut contexts: EguiContexts,
    progress: Res<LoadingProgress>,
    mut is_initialized: Local<bool>,
    mut frames_rendered: Local<u8>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) -> bevy::prelude::Result {
    let ctx = contexts.ctx_mut()?;

    if !*is_initialized {
        *is_initialized = true;
        egui_extras::install_image_loaders(ctx);
    }

    egui::Area::new("Left".into())
        .anchor(egui::Align2::LEFT_BOTTOM, [0., 0.])
        .show(ctx, |ui| {
            ui.image(egui::include_image!("../assets/loading_left.gif"));
        });

    egui::Window::new("Loading")
        .anchor(egui::Align2::CENTER_CENTER, [0., 0.])
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.);

                ui.heading("Loading...");
                ui.add_space(20.);

                let bar = egui::ProgressBar::new(progress.progress()).desired_width(300.);
                ui.add(bar);
                ui.add_space(10.);

                if progress.mesh < 24 {
                    ui.label(format!("Loading meshes ({}/{})", progress.mesh, 24));
                } else if progress.texture < 3 {
                    ui.label(format!("Loading textures ({}/3)", progress.texture));
                } else {
                    ui.label("Loading complete");
                }

                ui.add_space(10.);
            });
        });

    egui::Area::new("Right".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, [0., 0.])
        .show(ctx, |ui| {
            ui.image(egui::include_image!("../assets/loading_right.gif"));
        });

    if *state == GameState::PreLoading {
        *frames_rendered += 1;
        if *frames_rendered >= 3 {
            next_state.set(GameState::Loading);
        }
    }
    Ok(())
}
