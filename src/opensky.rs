use bevy::prelude::*;
use bevy_http_client::{
    HttpClient, HttpClientPlugin, HttpRequest, HttpResponse, HttpResponseError,
};
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
            .insert_resource(OpenSkyResource {
                username: self.username.clone(),
                password: self.password.clone(),
            })
            .add_event::<StateRequest>()
            .register_type::<StateVector>()
            .add_systems(
                Update,
                (get_all_states, handle_state_response, handle_error),
            );
    }
}

#[derive(Resource, Debug)]
pub struct OpenSkyResource {
    pub username: Option<String>,
    pub password: Option<String>,
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
    Option<f32>,
    Option<f32>,
    Option<f32>,
    bool,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<Vec<u64>>,
    Option<f32>,
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
    pub longitude: Option<f32>,
    /// WGS-84 latitude in decimal degrees. Can be null.
    pub latitude: Option<f32>,
    /// Barometric altitude in meters. Can be null.
    pub baro_altitude: Option<f32>,
    /// Boolean value which indicates if the position was retrieved from a surface position report.
    pub on_ground: bool,
    /// Velocity over ground in m/s. Can be null.
    pub velocity: Option<f32>,
    /// True track in decimal degrees clockwise from north (north=0°). Can be null.
    pub true_track: Option<f32>,
    /// Vertical rate in m/s. A positive value indicates that the airplane is climbing, a negative value indicates that it descends. Can be null.
    pub vertical_rate: Option<f32>,
    /// IDs of the receivers which contributed to this state vector. Is null if no filtering for sensor was used in the request.
    pub sensors: Option<Vec<u64>>,
    /// Geometric altitude in meters. Can be null.
    pub geo_altitude: Option<f32>,
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

fn get_all_states(mut events: EventReader<StateRequest>, mut state_req: EventWriter<HttpRequest>) {
    for req in events.read() {
        info!("GetAllStates: {:?}", req);
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
        let req = HttpClient::new().get(url).build();
        state_req.send(req);
    }
}

/// Handle the response from the OpenSky API
/// and spawn new entities or update existing ones.
fn handle_state_response(
    mut ev_response: EventReader<HttpResponse>,
    mut query: Query<&mut StateVector>,
    mut commands: Commands,
) {
    for response in ev_response.read() {
        let states = response
            .json::<StateResponse>()
            .unwrap()
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
                    *state = new_state;
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
}

fn handle_error(mut ev_error: EventReader<HttpResponseError>) {
    for error in ev_error.read() {
        error!("Error: {:?}", error);
    }
}
