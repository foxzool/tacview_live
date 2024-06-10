use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_activation::ActivationPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_tacview::{TACVIEW_CHANNEL, TacviewPlugin, TacviewResource};
use bevy_tacview::octopus::prelude::ListenTo;
use chrono::Utc;
use dotenvy::dotenv;

use tacview_live::opensky::OpenSkyPlugin;

fn main() {
    dotenv().expect(".env file not found");
    let username = std::env::var("OPENSKY_USERNAME").ok();
    let password = std::env::var("OPENSKY_PASSWORD").ok();
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(
            WorldInspectorPlugin::default().run_if(input_toggle_active(true, KeyCode::Escape)),
        )
        .add_plugins(OpenSkyPlugin { username, password })
        .add_plugins(ActivationPlugin)
        .add_plugins(TacviewPlugin)
        .add_systems(Startup, setup)
        .run()
}

fn setup(mut host_res: ResMut<TacviewResource>, mut commands: Commands) {
    *host_res = TacviewResource {
        title: "bevy tacview sample".to_string(),
        category: "test".to_string(),
        author: "zool".to_string(),
        reference_time: Some(Utc::now()),
        recording_time: Some(Utc::now()),
        briefing: "hit".to_string(),
        debriefing: "live".to_string(),
        comments: "no comment".to_string(),
        data_source: "Tacview".to_string(),
        data_recorder: "TacviewHost Example".to_string(),
    };
    commands.spawn((TACVIEW_CHANNEL, ListenTo::new("tcp://0.0.0.0:42674")));
}
