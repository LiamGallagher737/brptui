use crate::{Message, ThreadQuitToken};
use anyhow::anyhow;
use bevy_ecs::entity::Entity;
use bevy_remote::{
    builtin_methods::{
        BrpDestroyParams, BrpGetParams, BrpGetResponse, BrpListParams, BrpListResponse, BrpQuery,
        BrpQueryFilter, BrpQueryParams, BrpQueryResponse,
    },
    BrpPayload, BrpRequest,
};
use ratatui::{
    style::Stylize,
    text::{Line, Span},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::mpsc,
    time::{Duration, Instant},
};

pub const DEFAULT_SOCKET: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 15702);
pub const QUERY_COOLDOWN: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub struct EntityMeta {
    pub id: Entity,
    pub name: Option<String>,
}

impl EntityMeta {
    pub fn title(&self) -> Line {
        Line::from(vec![
            Span::raw(self.name()).bold(),
            Span::raw(" "),
            Span::raw(self.id.to_string()).dim(),
        ])
    }

    pub fn name(&self) -> String {
        self.name.clone().unwrap_or_else(|| String::from("Entity"))
    }
}

/// Query the connected BRP-enabled Bevy app every [`QUERY_COOLDOWN`] seconds.
///
/// Resulting [`Message`]s will be sent using the given [`mpsc::Sender`] to the
/// main thread to be handled.
pub fn handle_entity_querying(tx: mpsc::Sender<Message>, socket: &SocketAddr) {
    let mut last_time = Instant::now();
    loop {
        let params = BrpQueryParams {
            data: BrpQuery {
                option: vec!["bevy_core::name::Name".to_string()],
                ..Default::default()
            },
            filter: BrpQueryFilter::default(),
        };

        if let Ok(response) = query_request(socket, params) {
            let mut entities: Vec<_> = response
                .iter()
                .map(|row| EntityMeta {
                    id: row.entity,
                    name: row
                        .components
                        .get("bevy_core::name::Name")
                        .map(|name| name.get("name").unwrap().as_str().unwrap().to_string()),
                })
                .collect();

            entities.sort_by_key(|e| e.id);
            tx.send(Message::UpdateEntities(entities)).unwrap();
        } else {
            tx.send(Message::CommunicationFailed).unwrap();
        };

        // Sleep for the remaining time until the next query.
        std::thread::sleep(QUERY_COOLDOWN.saturating_sub(last_time.elapsed()));
        last_time = Instant::now();
    }
}

pub fn handle_components_querying(
    tx: mpsc::Sender<Message>,
    socket: &SocketAddr,
    entity: Entity,
    quit: ThreadQuitToken,
) {
    let Ok(components) = list_request(&socket, BrpListParams { entity }) else {
        tx.send(Message::CommunicationFailed).unwrap();
        return;
    };

    let params = BrpGetParams {
        entity,
        components,
        strict: false,
    };

    let mut last_time = Instant::now();
    loop {
        if quit.should_quit() {
            return;
        }

        if let Ok(BrpGetResponse::Lenient {
            components,
            errors: _,
        }) = get_request(&socket, params.clone())
        {
            tx.send(Message::UpdateComponents(components.into_iter().collect()))
                .unwrap();
        } else {
            tx.send(Message::CommunicationFailed).unwrap();
            return;
        }

        // Sleep for the remaining time until the next query.
        std::thread::sleep(QUERY_COOLDOWN.saturating_sub(last_time.elapsed()));
        last_time = Instant::now();
    }
}

/// Post a `bevy/get` request.
pub fn get_request(socket: &SocketAddr, params: BrpGetParams) -> anyhow::Result<BrpGetResponse> {
    request::<BrpGetParams, BrpGetResponse>(
        socket,
        bevy_remote::builtin_methods::BRP_GET_METHOD,
        params,
    )
}

/// Post a `bevy/query` request.
pub fn query_request(
    socket: &SocketAddr,
    params: BrpQueryParams,
) -> anyhow::Result<BrpQueryResponse> {
    request::<BrpQueryParams, BrpQueryResponse>(
        socket,
        bevy_remote::builtin_methods::BRP_QUERY_METHOD,
        params,
    )
}

/// Post a `bevy/destroy` request.
pub fn destroy_request(socket: &SocketAddr, params: BrpDestroyParams) -> anyhow::Result<()> {
    request::<BrpDestroyParams, ()>(
        socket,
        bevy_remote::builtin_methods::BRP_DESTROY_METHOD,
        params,
    )
}

/// Post a `bevy/list` request.
pub fn list_request(socket: &SocketAddr, params: BrpListParams) -> anyhow::Result<BrpListResponse> {
    request::<BrpListParams, BrpListResponse>(
        socket,
        bevy_remote::builtin_methods::BRP_LIST_METHOD,
        params,
    )
}

fn request<Params: Serialize, Response: DeserializeOwned>(
    socket: &SocketAddr,
    method: &str,
    params: Params,
) -> anyhow::Result<Response> {
    let request = BrpRequest {
        jsonrpc: String::from("2.0"),
        method: String::from(method),
        id: None,
        params: Some(serde_json::to_value(params)?),
    };

    let response = ureq::post(&format!("http://{socket}"))
        .send_json(request)?
        .into_json::<BrpResponse>()?;

    let body = match response.payload {
        BrpPayload::Result(value) => serde_json::from_value(value)?,
        BrpPayload::Error(err) => return Err(anyhow!("BrpPayload was an error: {err:?}")),
    };

    Ok(body)
}

/// A copy of [`bevy_remote::BrpResponse`] since it can't be deserialized due to `&'static str`.
#[derive(Debug, Deserialize, Clone)]
pub struct BrpResponse {
    /// The actual response payload.
    #[serde(flatten)]
    pub payload: BrpPayload,
}
