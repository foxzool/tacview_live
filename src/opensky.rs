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
}

#[derive(Debug, Default)]
pub struct BoundingBox {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
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

#[derive(Debug)]
pub struct StateVector {
    pub icao24: String,
    pub callsign: Option<String>,
    pub origin_country: String,
    pub time_position: Option<u64>,
    pub last_contact: u64,
    pub longitude: Option<f32>,
    pub latitude: Option<f32>,
    pub baro_altitude: Option<f32>,
    pub on_ground: bool,
    pub velocity: Option<f32>,
    pub true_track: Option<f32>,
    pub vertical_rate: Option<f32>,
    pub sensors: Option<Vec<u64>>,
    pub geo_altitude: Option<f32>,
    pub squawk: Option<String>,
    pub spi: bool,
    pub position_source: u8,
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

fn handle_state_response(mut ev_response: EventReader<HttpResponse>) {
    for response in ev_response.read() {
        let states = response
            .json::<StateResponse>()
            .unwrap()
            .states
            .into_iter()
            .map(StateVector::from)
            .collect::<Vec<_>>();
        info!("Response: {:?}", states);
    }
}

fn handle_error(mut ev_error: EventReader<HttpResponseError>) {
    for error in ev_error.read() {
        error!("Error: {:?}", error);
    }
}
