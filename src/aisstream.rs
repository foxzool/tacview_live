use std::ops::Deref;

use bevy::prelude::*;
use bevy_octopus::prelude::*;
use serde::{Deserialize, Serialize};

const AISSTREAM_CHANNEL: ChannelId = ChannelId("AIS");

pub struct AISStreamPlugin;

impl Plugin for AISStreamPlugin {
    fn build(&self, app: &mut App) {
        app
            // .add_transformer::<AuthMessage, JsonTransformer>(AISSTREAM_CHANNEL)
            .add_systems(Startup, setup)
            .add_systems(Update, (handle_connect, handle_raw_packet))
        ;
    }
}

#[derive(Resource)]
pub struct AISStreamResource {
    pub api_key: String,
}

fn setup(mut commands: Commands) {
    commands.spawn((AISSTREAM_CHANNEL, ConnectTo::new("wss://stream.aisstream.io/v0/stream")));
}

fn handle_connect(
    res: Res<AISStreamResource>,
    mut ev_node: EventReader<NetworkNodeEvent>, q_net_node: Query<&NetworkNode>) {
    for NetworkNodeEvent {
        node: entity,
        channel_id,
        event
    } in ev_node.read()
    {
        if *channel_id != AISSTREAM_CHANNEL {
            continue;
        }

        match event {
            NetworkEvent::Connected => {
                info!("Connected to {}", channel_id);
                let node = q_net_node.get(*entity).unwrap();
                let sub = serde_json::json!({
                    "APIKey": res.api_key,
                    "BoundingBoxes": [[[-180, -90], [180, 90]]]
                });
                println!("sub {}", sub);
                node.send_text(sub
                    .to_string())
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


fn handle_message_events(
    mut new_network_events: EventReader<NetworkData<AuthMessage>>,
) {
    for event in new_network_events.read() {
        let auth_message = event.deref();
        info!("Received: {:?}",  &auth_message);
    }
}

fn handle_raw_packet(q_server: Query<(&ChannelId, &NetworkNode)>) {
    for (channel_id, net_node) in q_server.iter() {
        if *channel_id == AISSTREAM_CHANNEL {
            while let Ok(Some(packet)) = net_node.recv_message_channel.receiver.try_recv() {
                info!("{} {} Received: {:?}", channel_id, net_node, packet.bytes);
            }
        }
    }
}
