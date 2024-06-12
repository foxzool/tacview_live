use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_activation::{ActivationPlugin, TimeoutEvent};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_octopus::plugin::OctopusPlugin;
use bevy_octopus::prelude::ListenTo;
use bevy_tacview::{TACVIEW_CHANNEL, TacviewPlugin, TacviewResource};
use bevy_tacview::systems::ObjectNeedSync;
use chrono::Utc;
use dotenvy::dotenv;

pub mod aisstream;
pub mod opensky;

fn main() {
    dotenv().expect(".env file not found");
    let username = std::env::var("OPENSKY_USERNAME").ok();
    let password = std::env::var("OPENSKY_PASSWORD").ok();
    let api_key = std::env::var("AISSTREAM_KEY").unwrap();
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "bevy_octopus=trace,tacview_live=debug".to_string(),
            ..default()
        }))
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(opensky::OpenSkyPlugin { username, password })
        .add_plugins(ActivationPlugin)
        .add_plugins(OctopusPlugin)
        .add_plugins(TacviewPlugin)
        .insert_resource(aisstream::AISStreamResource { api_key })
        .add_plugins(aisstream::AISStreamPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, watch_timeout)
        .run()
}
//
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

fn watch_timeout(mut ev_timeout: EventReader<TimeoutEvent>, mut commands: Commands) {
    for timeout in ev_timeout.read() {
        debug!("Timeout: {:?}", timeout);
        commands.entity(timeout.0).insert(ObjectNeedSync::Destroy);
    }
}
