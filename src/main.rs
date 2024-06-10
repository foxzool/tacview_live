use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use tacview_live::opensky::{BoundingBox, OpenSkyPlugin, StateRequest};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(
            WorldInspectorPlugin::default().run_if(input_toggle_active(true, KeyCode::Escape)),
        )
        .add_plugins(OpenSkyPlugin::default())
        .add_systems(Startup, setup)
        .run()
}

fn setup(mut get_all_state_ev: EventWriter<StateRequest>) {
    get_all_state_ev.send(StateRequest {
        bounding_box: Some(BoundingBox {
            min_lat: 3.2063329870791444,
            max_lat: 29.477861195816843,
            min_lon: 97.4267578125,
            max_lon: 141.48193359375003,
        }),

        ..default()
    });
}