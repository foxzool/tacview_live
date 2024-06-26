use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy_activation::ActiveState;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_octopus::prelude::*;
use bevy_tacview::record::{Coords, Property, PropertyList, Tag};
use bevy_tacview::systems::ObjectNeedSync;
use chrono::NaiveDateTime;
use serde::{Deserialize, Deserializer, Serialize};

use crate::opensky::StateVector;

const AISSTREAM_CHANNEL: ChannelId = ChannelId("AIS");

pub struct AISStreamPlugin;

impl Plugin for AISStreamPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MSSIIndex>()
            .register_type::<MetaData>()
            .register_type::<PositionReport>()
            .register_type::<MSSIIndex>()
            .add_plugins(ResourceInspectorPlugin::<MSSIIndex>::default())
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_connect, handle_raw_packet))
            .add_systems(Update, (watch_added, watch_changed));
    }
}

/// store the api key for AISStream
#[derive(Resource)]
pub struct AISStreamResource {
    pub api_key: String,
}

/// setup the connection to AISStream
fn setup(mut commands: Commands) {
    commands.spawn((
        AISSTREAM_CHANNEL,
        ConnectTo::new("wss://stream.aisstream.io/v0/stream"),
    ));
}

fn handle_connect(
    res: Res<AISStreamResource>,
    mut ev_node: EventReader<NetworkNodeEvent>,
    q_net_node: Query<&NetworkNode>,
) {
    for NetworkNodeEvent {
        node: entity,
        channel_id,
        event,
    } in ev_node.read()
    {
        if *channel_id != AISSTREAM_CHANNEL {
            continue;
        }

        match event {
            NetworkEvent::Connected => {
                info!("{channel_id} Connected");
                let node = q_net_node.get(*entity).unwrap();
                let sub = serde_json::json!({
                    "APIKey": res.api_key,
                    "BoundingBoxes": [[[3.2063329870791444, 97.4267578125], [29.477861195816843, 141.48193359375003 ]]],
                    // "BoundingBoxes": [[[97.4267578125, 3.2063329870791444], [141.48193359375003, 29.477861195816843]]],
                    // "FilterMessageTypes": ["PositionReport"]
                });
                node.send_text(sub.to_string())
            }
            NetworkEvent::Disconnected => {
                info!("Disconnected from {}", channel_id);
            }
            NetworkEvent::Listen => {}
            NetworkEvent::Error(error) => {
                error!("Error on {}: {:?}", channel_id, error);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
enum AuthMessage {
    AuthError(AuthError),
    Message(Message),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AuthError {
    pub error: String,
}

type Message = serde_json::Value;

#[derive(Resource, Default, Deref, Reflect, DerefMut)]
struct MSSIIndex(HashMap<i32, Entity>);

fn handle_raw_packet(
    q_server: Query<(&ChannelId, &NetworkNode)>,
    mut commands: Commands,
    mut q_vessels: Query<(&mut MetaData, )>,
    mut mssi_index: ResMut<MSSIIndex>,
) {
    for (channel_id, net_node) in q_server.iter() {
        if *channel_id == AISSTREAM_CHANNEL {
            while let Ok(Some(packet)) = net_node.recv_message_channel.receiver.try_recv() {
                let message: AuthMessage = serde_json::from_slice(&packet.bytes).unwrap();
                // info!("Received: {:?}", message);
                match message {
                    AuthMessage::AuthError(e) => {
                        error!("AuthError: {:?}", e.error);
                    }
                    AuthMessage::Message(m) => {
                        // let position_report: PositionReport =
                        //     serde_json::from_value(m["Message"]["PositionReport"].clone()).unwrap();
                        // trace!("position_report: {:?}", position_report);
                        let meta_data: MetaData =
                            serde_json::from_value(m["MetaData"].clone()).unwrap();
                        trace!("meta_data: {:?}", meta_data);
                        if let Some(entity) = mssi_index.get(&meta_data.mmsi) {
                            if let Ok((mut meta_data_comp, )) =
                                q_vessels.get_mut(*entity)
                            {
                                meta_data_comp.set_if_neq(meta_data);
                                // position_report_comp.set_if_neq(position_report);
                            }
                        } else {
                            let mssi = meta_data.mmsi;
                            let entity = commands.spawn((meta_data, )).id();
                            mssi_index.insert(mssi, entity);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize, Component, Reflect, PartialEq)]
struct PositionReport {
    #[serde(rename = "MessageID")]
    message_id: i32,
    #[serde(rename = "RepeatIndicator")]
    repeat_indicator: i32,
    #[serde(rename = "UserID")]
    user_id: i32,
    #[serde(rename = "Valid")]
    valid: bool,
    #[serde(rename = "NavigationalStatus")]
    navigational_status: i32,
    #[serde(rename = "RateOfTurn")]
    rate_of_turn: i32,
    #[serde(rename = "Sog")]
    sog: f64,
    #[serde(rename = "PositionAccuracy")]
    position_accuracy: bool,
    #[serde(rename = "Longitude")]
    longitude: f64,
    #[serde(rename = "Latitude")]
    latitude: f64,
    #[serde(rename = "Cog")]
    cog: f64,
    #[serde(rename = "TrueHeading")]
    true_heading: i32,
    #[serde(rename = "Timestamp")]
    timestamp: i32,
    #[serde(rename = "SpecialManoeuvreIndicator")]
    special_manoeuvre_indicator: i32,
    #[serde(rename = "Spare")]
    spare: i32,
    #[serde(rename = "Raim")]
    raim: bool,
    #[serde(rename = "CommunicationState")]
    communication_state: i32,
}

#[derive(Debug, Deserialize, Component, Reflect, PartialEq)]
struct MetaData {
    #[serde(rename = "MMSI")]
    mmsi: i32,
    #[serde(rename = "ShipName")]
    ship_name: String,
    longitude: f64,
    latitude: f64,
    // #[serde(deserialize_with = "decode_time_utc")]
    time_utc: String,
}

#[allow(dead_code)]
fn decode_time_utc<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let naive_dt = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S%.f %z %Z")
        .expect("Failed to parse date time");

    Ok(naive_dt)
}

fn watch_added(query: Query<(Entity, &MetaData), (Added<MetaData>,)>, mut commands: Commands) {
    for (e, meta_data) in query.iter() {
        trace!("Added: {} {}", meta_data.mmsi, meta_data.ship_name);
        let coord = to_coords(meta_data);
        let props = to_props(meta_data);

        commands.entity(e).insert((
            coord,
            PropertyList(props),
            ObjectNeedSync::Spawn,
            ActiveState::always(),
        ));
    }
}

fn watch_changed(
    mut query: Query<
        (
            Entity,
            &MetaData,
            &mut Coords,
            &mut PropertyList,
            &mut ActiveState,
        ),
        Changed<StateVector>,
    >,
    mut commands: Commands,
) {
    for (entity, meta_data, mut coords, mut props_list, mut active_state) in query.iter_mut() {
        coords.set_if_neq(to_coords(&meta_data));
        props_list.set_if_neq(PropertyList(to_props(&meta_data)));
        active_state.toggle();
        commands.entity(entity).insert(ObjectNeedSync::Update);
    }
}

fn to_coords(position_report: &MetaData) -> Coords {
    Coords {
        longitude: Some(position_report.longitude),
        latitude: Some(position_report.latitude),
        altitude: Some(0.0),
        u: None,
        v: None,
        roll: None,
        pitch: None,
        yaw: None,
        heading: None,
    }
}

fn to_props(meta_data: &MetaData) -> Vec<Property> {
    let list = vec![
        Property::CallSign(meta_data.ship_name.clone()),
        Property::Type(HashSet::from_iter([Tag::Watercraft])),
    ];

    list
}
