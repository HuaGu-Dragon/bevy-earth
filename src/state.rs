use bevy::state::state::States;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    PreLoading,
    Loading,
    PostLoading,
    Playing,
}
