use std::time::Duration;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bevy::prelude::*;
use bevy::time::common_conditions::on_real_timer;
use bevy_activation::{ActiveState, TimeoutEvent};
use bevy_http_client::{
    HttpClient, HttpClientPlugin, HttpRequest, HttpResponse, HttpResponseError,
};
use bevy_tacview::record::{Coords, Property, PropertyList};
use bevy_tacview::systems::ObjectNeedSync;
use serde::Deserialize;
use url::Url;

#[derive(Default)]
pub struct OpenSkyPlugin {
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Plugin for OpenSkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HttpClientPlugin)
            .insert_resource(OpenSkyResource::new(&self.username, &self.password))
            .add_event::<StateRequest>()
            .register_type::<StateVector>()
            .add_systems(
                Update,
                (
                    refresh_states.run_if(on_real_timer(Duration::from_secs(10))),
                    get_all_states,
                    handle_state_response,
                    handle_error,
                    watch_added,
                    watch_changed,
                    watch_timeout,
                ),
            );
    }
}

#[derive(Resource, Debug)]
pub struct OpenSkyResource {
    pub auth: Option<String>,
}

impl OpenSkyResource {
    pub fn new(username: &Option<String>, password: &Option<String>) -> Self {
        let auth = if let (Some(username), Some(password)) = (username, password) {
            Some(BASE64_STANDARD.encode(&format!("{}:{}", username, password)))
        } else {
            None
        };
        Self { auth }
    }
}

#[derive(Event, Debug, Default)]
pub struct StateRequest {
    /// The time in seconds since epoch (Unix time stamp to retrieve states for. Current time will be used if omitted.
    pub time: Option<u64>,
    /// One or more ICAO24 transponder addresses represented by a hex string (e.g. abc9f3). To filter multiple ICAO24 append the property once for each address. If omitted, the state vectors of all aircraft are returned.
    pub icao24: Option<String>,
    pub bounding_box: Option<BoundingBox>,
    /// if request the  state vector category, set to 1
    pub extended: Option<u8>,
}

#[derive(Debug, Default)]
pub struct BoundingBox {
    /// lower bound for the latitude in decimal degrees
    pub min_lat: f64,
    /// upper bound for the latitude in decimal degrees
    pub max_lat: f64,
    /// lower bound for the longitude in decimal degrees
    pub min_lon: f64,
    /// upper bound for the longitude in decimal degrees
    pub max_lon: f64,
}

#[derive(Debug, Deserialize)]
pub struct StateResponse {
    pub time: u64,
    pub states: Vec<InnerStateVector>,
}

#[derive(Deserialize, Debug)]
pub struct InnerStateVector(
    String,
    Option<String>,
    String,
    Option<u64>,
    u64,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    bool,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<Vec<u64>>,
    Option<f64>,
    Option<String>,
    bool,
    u8,
    // Option<i32>
);

#[derive(Debug, Component, Reflect)]
pub struct StateVector {
    /// Unique ICAO 24-bit address of the transponder in hex string representation.
    pub icao24: String,
    /// Callsign of the vehicle (8 chars). Can be null if no callsign has been received.
    pub callsign: Option<String>,
    /// Country name inferred from the ICAO 24-bit address.
    pub origin_country: String,
    /// Unix timestamp (seconds) for the last position update. Can be null if no position report was received by OpenSky within the past 15s.
    pub time_position: Option<u64>,
    /// Unix timestamp (seconds) for the last update in general. This field is updated for any new, valid message received from the transponder.
    pub last_contact: u64,
    /// WGS-84 longitude in decimal degrees. Can be null.
    pub longitude: Option<f64>,
    /// WGS-84 latitude in decimal degrees. Can be null.
    pub latitude: Option<f64>,
    /// Barometric altitude in meters. Can be null.
    pub baro_altitude: Option<f64>,
    /// Boolean value which indicates if the position was retrieved from a surface position report.
    pub on_ground: bool,
    /// Velocity over ground in m/s. Can be null.
    pub velocity: Option<f64>,
    /// True track in decimal degrees clockwise from north (north=0°). Can be null.
    pub true_track: Option<f64>,
    /// Vertical rate in m/s. A positive value indicates that the airplane is climbing, a negative value indicates that it descends. Can be null.
    pub vertical_rate: Option<f64>,
    /// IDs of the receivers which contributed to this state vector. Is null if no filtering for sensor was used in the request.
    pub sensors: Option<Vec<u64>>,
    /// Geometric altitude in meters. Can be null.
    pub geo_altitude: Option<f64>,
    /// The transponder code aka Squawk. Can be null.
    pub squawk: Option<String>,
    /// Whether flight status indicates special purpose indicator.
    pub spi: bool,
    /// Origin of this state’s position.
    ///
    /// * 0 = ADS-B
    ///
    /// * 1 = ASTERIX
    ///
    /// * 2 = MLAT
    ///
    /// * 3 = FLARM
    pub position_source: u8,
    ////Aircraft category.
    ///
    /// * 0 = No information at all
    ///
    /// * 1 = No ADS-B Emitter Category Information
    ///
    /// * 2 = Light (< 15500 lbs)
    ///
    /// * 3 = Small (15500 to 75000 lbs)
    ///
    /// * 4 = Large (75000 to 300000 lbs)
    ///
    /// * 5 = High Vortex Large (aircraft such as B-757)
    ///
    /// * 6 = Heavy (> 300000 lbs)
    ///
    /// * 7 = High Performance (> 5g acceleration and 400 kts)
    ///
    /// * 8 = Rotorcraft
    ///
    /// 9 = Glider / sailplane
    ///
    /// * 10 = Lighter-than-air
    ///
    /// * 11 = Parachutist / Skydiver
    ///
    /// * 12 = Ultralight / hang-glider / paraglider
    ///
    /// * 13 = Reserved
    ///
    /// * 14 = Unmanned Aerial Vehicle
    ///
    /// * 15 = Space / Trans-atmospheric vehicle
    ///
    /// * 16 = Surface Vehicle – Emergency Vehicle
    //
    /// * 17 = Surface Vehicle – Service Vehicle
    ///
    /// * 18 = Point Obstacle (includes tethered balloons)
    ///
    /// * 19 = Cluster Obstacle
    ///
    /// * 20 = Line Obstacle
    pub category: Option<u32>,
}

impl PartialEq for StateVector {
    fn eq(&self, other: &Self) -> bool {
        self.icao24 == other.icao24 && self.time_position == other.time_position
    }
}

impl From<InnerStateVector> for StateVector {
    fn from(inner: InnerStateVector) -> Self {
        StateVector {
            icao24: inner.0,
            callsign: inner.1,
            origin_country: inner.2,
            time_position: inner.3,
            last_contact: inner.4,
            longitude: inner.5,
            latitude: inner.6,
            baro_altitude: inner.7,
            on_ground: inner.8,
            velocity: inner.9,
            true_track: inner.10,
            vertical_rate: inner.11,
            sensors: inner.12,
            geo_altitude: inner.13,
            squawk: inner.14,
            spi: inner.15,
            position_source: inner.16,
            category: None,
        }
    }
}

fn refresh_states(mut state_req: EventWriter<StateRequest>) {
    state_req.send(StateRequest {
        bounding_box: Some(BoundingBox {
            min_lat: 3.2063329870791444,
            max_lat: 29.477861195816843,
            min_lon: 97.4267578125,
            max_lon: 141.48193359375003,
        }),

        ..default()
    });
}

fn get_all_states(
    mut events: EventReader<StateRequest>,
    mut state_req: EventWriter<HttpRequest>,
    opensky_res: Res<OpenSkyResource>,
) {
    for req in events.read() {
        debug!("request state: {:?}", req);
        let api_url = "https://opensky-network.org/api/states/all";
        let mut url = Url::parse(api_url).unwrap();
        if let Some(time) = req.time {
            url.query_pairs_mut().append_pair("time", &time.to_string());
        }
        if let Some(ico24) = req.icao24.as_ref() {
            url.query_pairs_mut().append_pair("icao24", &ico24);
        }
        if let Some(bbox) = req.bounding_box.as_ref() {
            url.query_pairs_mut()
                .append_pair("lamin", &bbox.min_lat.to_string())
                .append_pair("lomin", &bbox.min_lon.to_string())
                .append_pair("lamax", &bbox.max_lat.to_string())
                .append_pair("lomax", &bbox.max_lon.to_string());
        }

        let req = if let Some(auth) = opensky_res.auth.as_ref() {
            println!("auth : {}", auth);
            HttpClient::new()
                .headers(&[
                    ("Content-Type", "application/json"),
                    ("Accept", "*/*"),
                    ("Authorization", format!("Basic {}", auth).as_str()),
                ])
                .get(url)
                .build()
        } else {
            HttpClient::new().get(url).build()
        };
        state_req.send(req);
    }
}

/// Handle the response from the OpenSky API
/// and spawn new entities or update existing ones.
#[allow(unused_assignments)]
fn handle_state_response(
    mut ev_response: EventReader<HttpResponse>,
    mut query: Query<&mut StateVector>,
    mut commands: Commands,
) {
    for response in ev_response.read() {
        match response.json::<StateResponse>() {
            Ok(resp_json) => {
                let states = resp_json
                    .states
                    .into_iter()
                    .map(StateVector::from)
                    .collect::<Vec<_>>();
                trace!("Response: {:?}", states);

                let mut new_batches = vec![];
                'a: for new_state in states {
                    let mut not_find = true;
                    for mut state in query.iter_mut() {
                        if state.icao24 == new_state.icao24 {
                            state.set_if_neq(new_state);
                            not_find = false;
                            continue 'a;
                        }
                    }

                    if not_find {
                        new_batches.push(new_state);
                    }
                }
                commands.spawn_batch(new_batches);
            }
            Err(_e) => {
                error!("Error: {:?}", response.text());
                return;
            }
        }
    }
}

fn handle_error(mut ev_error: EventReader<HttpResponseError>) {
    for error in ev_error.read() {
        error!("Error: {:?}", error);
    }
}

fn watch_added(query: Query<(Entity, &StateVector), Added<StateVector>>, mut commands: Commands) {
    for (e, state) in query.iter() {
        debug!("Added: {:?}", state);
        let coord = to_coords(state);
        let props = to_props(state);

        commands.entity(e).insert((
            coord,
            PropertyList(props),
            ObjectNeedSync::Spawn,
            ActiveState::new(Duration::from_secs(20)),
        ));
    }
}

fn watch_changed(
    mut query: Query<
        (
            Entity,
            &StateVector,
            &mut Coords,
            &mut PropertyList,
            &mut ActiveState,
        ),
        Changed<StateVector>,
    >,
    mut commands: Commands,
) {
    for (entity, state, mut coords, mut props_list, mut active_state) in query.iter_mut() {
        debug!("Changed: {:?} after {}", state.icao24, state.last_contact);
        coords.set_if_neq(to_coords(state));
        props_list.set_if_neq(PropertyList(to_props(state)));
        active_state.toggle();
        commands.entity(entity).insert(ObjectNeedSync::Update);
    }
}

fn watch_timeout(mut ev_timeout: EventReader<TimeoutEvent>, mut commands: Commands) {
    for timeout in ev_timeout.read() {
        info!("Timeout: {:?}", timeout);
        commands.entity(timeout.0).insert(ObjectNeedSync::Destroy);
    }
}

fn to_coords(state: &StateVector) -> Coords {
    Coords {
        longitude: state.longitude,
        latitude: state.latitude,
        altitude: state.baro_altitude,
        u: None,
        v: None,
        roll: Some(0.0),
        pitch: Some(0.0),
        yaw: state.true_track,
        heading: None,
    }
}

fn to_props(state: &StateVector) -> Vec<Property> {
    let mut list = vec![
        Property::Name(state.icao24.clone()),
        Property::ICAO24(state.icao24.clone()),
        Property::Country(state.origin_country.clone()),
    ];

    if let Some(call_sign) = state.callsign.as_ref() {
        list.push(Property::CallSign(call_sign.clone()));
    }

    list
}
