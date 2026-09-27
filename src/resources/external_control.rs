//! Loopback HTTP input.  The HTTP thread never accesses the ECS world.
use crate::{
    resources::{
        launch_profile::LaunchProfile,
        stone_type::{StoneCapabilities, StoneType},
    },
    scenes::stage::{
        components::{DigLimit, StoneIndex, StoneRune},
        systems::{
            PlaceState, ScriptEditorState, StageProgressionState, StoneAppendCommandMessage,
            StoneCommandState,
        },
    },
    util::script_types::{MoveDirection, ScriptCommand},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError},
    },
    thread,
};
use tokio::{net::TcpListener, sync::oneshot, time::Duration};
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer};

pub const MAX_INGRESS: usize = 64;
const MAX_RECORDS: usize = 128;
#[derive(Clone, Debug)]
pub enum ExternalRequest {
    Start,
    Stop,
    Reset,
    Command {
        stone: usize,
        command: ScriptCommand,
    },
}
#[derive(Clone, Debug)]
pub struct ExternalCommand {
    pub id: u64,
    pub generation: u64,
    pub request: ExternalRequest,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ControlSnapshot {
    pub generation: u64,
    pub active_stage: Option<usize>,
    pub external_owner: bool,
    pub stones: Vec<StoneSnapshot>,
}
#[derive(Clone, Debug, Serialize)]
pub struct StoneSnapshot {
    pub index: usize,
    pub capabilities: Vec<String>,
    pub x: f32,
    pub y: f32,
    pub busy: bool,
    pub queued: usize,
    pub dig_remaining: Option<u32>,
    pub place_remaining: Option<u32>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ActionRecord {
    pub id: u64,
    pub generation: u64,
    pub status: String,
    pub detail: Option<String>,
}
#[derive(Default)]
struct Records(VecDeque<ActionRecord>);
#[derive(Resource, Clone)]
pub struct ExternalControlBridge {
    ingress: SyncSender<ExternalCommand>,
    snapshot: Arc<Mutex<ControlSnapshot>>,
    records: Arc<Mutex<Records>>,
    ids: Arc<AtomicU64>,
}
#[derive(Resource)]
pub struct ExternalControlReceiver(pub Receiver<ExternalCommand>);
#[derive(Resource, Default)]
pub struct ExternalControlState {
    pub generation: u64,
    pub owner: bool,
    active: HashMap<usize, (u64, u64, bool)>,
}
impl ExternalControlBridge {
    fn put(&self, r: ActionRecord) {
        let mut q = self.records.lock().unwrap();
        q.0.retain(|v| v.id != r.id);
        q.0.push_back(r);
        while q.0.len() > MAX_RECORDS {
            q.0.pop_front();
        }
    }
    pub fn finish(&self, id: u64, generation: u64, status: &str, detail: Option<String>) {
        self.put(ActionRecord {
            id,
            generation,
            status: status.into(),
            detail,
        })
    }
    fn submit(&self, generation: u64, request: ExternalRequest) -> Result<ActionRecord, ApiError> {
        let id = self.ids.fetch_add(1, Ordering::Relaxed);
        let r = ActionRecord {
            id,
            generation,
            status: "queued".into(),
            detail: None,
        };
        match self.ingress.try_send(ExternalCommand {
            id,
            generation,
            request,
        }) {
            Ok(()) => {
                self.put(r.clone());
                Ok(r)
            }
            Err(TrySendError::Full(_)) => Err(ApiError(429, "queue_full")),
            Err(TrySendError::Disconnected(_)) => Err(ApiError(503, "server_stopped")),
        }
    }
}
pub struct ExternalControlServer {
    shutdown: Option<oneshot::Sender<()>>,
    join: Option<thread::JoinHandle<()>>,
}
impl ExternalControlServer {
    pub fn stop(mut self) {
        if let Some(s) = self.shutdown.take() {
            let _ = s.send(());
        }
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}
#[derive(Clone)]
struct Http {
    bridge: ExternalControlBridge,
    token: Arc<str>,
}
#[derive(Deserialize)]
struct CommandBody {
    generation: u64,
    command: String,
    direction: Option<String>,
    duration: Option<f32>,
}
#[derive(Serialize)]
struct Error {
    error: &'static str,
}
struct ApiError(u16, &'static str);
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::from_u16(self.0).unwrap(),
            Json(Error { error: self.1 }),
        )
            .into_response()
    }
}
fn auth(h: &HeaderMap, s: &Http) -> Result<(), ApiError> {
    if h.contains_key(header::ORIGIN) {
        return Err(ApiError(403, "cross_origin_forbidden"));
    }
    let expected = format!("Bearer {}", s.token);
    if h.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(ApiError(401, "unauthorized"))
    }
}
async fn state(State(s): State<Http>, h: HeaderMap) -> Result<Json<ControlSnapshot>, ApiError> {
    auth(&h, &s)?;
    Ok(Json(s.bridge.snapshot.lock().unwrap().clone()))
}
async fn action(
    State(s): State<Http>,
    h: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ActionRecord>, ApiError> {
    auth(&h, &s)?;
    s.bridge
        .records
        .lock()
        .unwrap()
        .0
        .iter()
        .find(|v| v.id == id)
        .cloned()
        .map(Json)
        .ok_or(ApiError(404, "action_not_found"))
}
async fn session(
    State(s): State<Http>,
    h: HeaderMap,
    Path(v): Path<String>,
) -> Result<(StatusCode, Json<ActionRecord>), ApiError> {
    auth(&h, &s)?;
    let r = match v.as_str() {
        "start" => ExternalRequest::Start,
        "stop" => ExternalRequest::Stop,
        "reset" => ExternalRequest::Reset,
        _ => return Err(ApiError(404, "unknown_session_action")),
    };
    let g = s.bridge.snapshot.lock().unwrap().generation;
    Ok((StatusCode::ACCEPTED, Json(s.bridge.submit(g, r)?)))
}
fn command(b: CommandBody) -> Result<ScriptCommand, ApiError> {
    let dir = || {
        b.direction
            .as_deref()
            .and_then(MoveDirection::from_str)
            .ok_or(ApiError(422, "invalid_direction"))
    };
    match b.command.as_str() {
        "move" => Ok(ScriptCommand::Move(dir()?)),
        "dig" => Ok(ScriptCommand::Dig(dir()?)),
        "place" => Ok(ScriptCommand::Place(dir()?)),
        "sleep" => {
            let n = b.duration.ok_or(ApiError(422, "missing_duration"))?;
            if !n.is_finite() || !(0.0..=60.0).contains(&n) {
                Err(ApiError(422, "invalid_duration"))
            } else {
                Ok(ScriptCommand::Sleep(n))
            }
        }
        _ => Err(ApiError(422, "unsupported_command")),
    }
}
async fn submit(
    State(s): State<Http>,
    h: HeaderMap,
    Path(stone): Path<usize>,
    Json(b): Json<CommandBody>,
) -> Result<(StatusCode, Json<ActionRecord>), ApiError> {
    auth(&h, &s)?;
    let generation = b.generation;
    let c = command(b)?;
    let snap = s.bridge.snapshot.lock().unwrap().clone();
    if snap.active_stage.is_none() {
        return Err(ApiError(409, "no_active_stage"));
    }
    if !snap.external_owner {
        return Err(ApiError(409, "external_session_not_started"));
    }
    if generation != snap.generation {
        return Err(ApiError(409, "stale_generation"));
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(
            s.bridge
                .submit(generation, ExternalRequest::Command { stone, command: c })?,
        ),
    ))
}
pub fn start(
    profile: &LaunchProfile,
) -> Result<
    Option<(
        ExternalControlBridge,
        ExternalControlReceiver,
        ExternalControlServer,
    )>,
    String,
> {
    if !profile.external_control {
        return Ok(None);
    }
    let token = profile
        .external_control_token
        .clone()
        .ok_or("--external-control requires KEYSTONE_EXTERNAL_CONTROL_TOKEN")?;
    let (tx, rx) = std::sync::mpsc::sync_channel(MAX_INGRESS);
    let bridge = ExternalControlBridge {
        ingress: tx,
        snapshot: Arc::new(Mutex::new(ControlSnapshot::default())),
        records: Arc::new(Mutex::new(Records::default())),
        ids: Arc::new(AtomicU64::new(1)),
    };
    let h = Http {
        bridge: bridge.clone(),
        token: Arc::from(token),
    };
    let (stop, stop_rx) = oneshot::channel();
    let port = profile.external_control_port.unwrap_or(38473);
    let join = thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let listener =
                    TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
                        .await
                        .expect("loopback bind");
                let app = Router::new()
                    .route("/v1/state", get(state))
                    .route("/v1/actions/{id}", get(action))
                    .route("/v1/session/{action}", post(session))
                    .route("/v1/stones/{stone}/commands", post(submit))
                    .layer(RequestBodyLimitLayer::new(8192))
                    .layer(TimeoutLayer::new(Duration::from_secs(3)))
                    .with_state(h);
                let server = axum::serve(listener, app);
                tokio::select! {_=server=>{},_=stop_rx=>{}}
            });
    });
    Ok(Some((
        bridge,
        ExternalControlReceiver(rx),
        ExternalControlServer {
            shutdown: Some(stop),
            join: Some(join),
        },
    )))
}

pub fn drain_external_commands(
    bridge: Res<ExternalControlBridge>,
    receiver: Res<ExternalControlReceiver>,
    mut control: ResMut<ExternalControlState>,
    mut editor: ResMut<ScriptEditorState>,
    mut append: MessageWriter<StoneAppendCommandMessage>,
    mut stones: Query<(&StoneIndex, &StoneType, &mut StoneCommandState), With<StoneRune>>,
) {
    while let Ok(request) = receiver.0.try_recv() {
        if request.generation != control.generation {
            bridge.finish(
                request.id,
                request.generation,
                "rejected",
                Some("stale_generation".into()),
            );
            continue;
        }
        match request.request {
            ExternalRequest::Start => {
                control.owner = true;
                editor.controls_enabled = false;
                editor.active_programs.clear();
                for (_, _, mut state) in &mut stones {
                    state.clear_commands();
                }
                bridge.finish(request.id, control.generation, "complete", None);
            }
            ExternalRequest::Stop => {
                control.owner = false;
                for (_, _, mut state) in &mut stones {
                    state.clear_commands();
                }
                for (_, (id, generation, _)) in control.active.drain() {
                    bridge.finish(id, generation, "rejected", Some("session_stopped".into()));
                }
                bridge.finish(request.id, control.generation, "complete", None);
            }
            ExternalRequest::Reset => {
                editor.controls_enabled = false;
                editor.active_programs.clear();
                editor.pending_player_reset = true;
                for (_, (id, generation, _)) in control.active.drain() {
                    bridge.finish(id, generation, "rejected", Some("reset".into()));
                }
                for (_, _, mut state) in &mut stones {
                    state.clear_commands();
                }
                bridge.finish(request.id, control.generation, "complete", None);
            }
            ExternalRequest::Command { stone, command } => {
                let Some((_, kind, state)) =
                    stones.iter_mut().find(|(index, _, _)| index.0 == stone)
                else {
                    bridge.finish(
                        request.id,
                        control.generation,
                        "rejected",
                        Some("stone_not_found".into()),
                    );
                    continue;
                };
                let name = match command {
                    ScriptCommand::Move(_) => "move",
                    ScriptCommand::Sleep(_) => "sleep",
                    ScriptCommand::Dig(_) => "dig",
                    ScriptCommand::Place(_) => "place",
                };
                if !control.owner {
                    bridge.finish(
                        request.id,
                        control.generation,
                        "rejected",
                        Some("external_session_not_started".into()),
                    );
                } else if state.is_busy() || control.active.contains_key(&stone) {
                    bridge.finish(
                        request.id,
                        control.generation,
                        "rejected",
                        Some("stone_busy".into()),
                    );
                } else if !StoneCapabilities::default()
                    .get_capabilities(*kind)
                    .is_some_and(|c| c.contains(name))
                {
                    bridge.finish(
                        request.id,
                        control.generation,
                        "rejected",
                        Some("unsupported_command".into()),
                    );
                } else {
                    append.write(StoneAppendCommandMessage {
                        stone_index: stone,
                        command,
                    });
                    control
                        .active
                        .insert(stone, (request.id, control.generation, false));
                    bridge.finish(request.id, control.generation, "running", None);
                }
            }
        }
    }
    let done: Vec<_> = control
        .active
        .iter()
        .filter_map(|(&stone, &(id, generation, started))| {
            stones
                .iter()
                .find(|(index, _, _)| index.0 == stone)
                .filter(|(_, _, state)| started && !state.is_busy())
                .map(|_| (stone, id, generation))
        })
        .collect();
    for (stone, id, generation) in done {
        control.active.remove(&stone);
        bridge.finish(id, generation, "complete", None);
    }
    for (&stone, entry) in &mut control.active {
        if stones
            .iter()
            .any(|(index, _, state)| index.0 == stone && state.is_busy())
        {
            entry.2 = true;
        }
    }
}

pub fn publish_external_snapshot(
    bridge: Res<ExternalControlBridge>,
    control: Res<ExternalControlState>,
    progression: Res<StageProgressionState>,
    place: Option<Res<PlaceState>>,
    stones: Query<
        (
            &StoneIndex,
            &StoneType,
            &Transform,
            &StoneCommandState,
            &DigLimit,
        ),
        With<StoneRune>,
    >,
) {
    let mut snapshot = ControlSnapshot {
        generation: control.generation,
        active_stage: progression.current_stage().map(|s| s.id.0),
        external_owner: control.owner,
        stones: Vec::new(),
    };
    let place_remaining = place.as_ref().and_then(|p| p.remaining);
    for (index, kind, transform, state, dig) in &stones {
        let mut capabilities: Vec<_> = StoneCapabilities::default()
            .get_capabilities(*kind)
            .into_iter()
            .flatten()
            .cloned()
            .collect();
        capabilities.sort();
        snapshot.stones.push(StoneSnapshot {
            index: index.0,
            capabilities,
            x: transform.translation.x,
            y: transform.translation.y,
            busy: state.is_busy(),
            queued: state.queue.len(),
            dig_remaining: dig.0,
            place_remaining,
        });
    }
    snapshot.stones.sort_by_key(|s| s.index);
    *bridge.snapshot.lock().unwrap() = snapshot;
}

pub fn invalidate_external_generation(
    mut control: ResMut<ExternalControlState>,
    bridge: Option<Res<ExternalControlBridge>>,
) {
    control.generation = control.generation.wrapping_add(1);
    control.owner = false;
    for (_, (id, generation, _)) in control.active.drain() {
        if let Some(bridge) = bridge.as_ref() {
            bridge.finish(id, generation, "rejected", Some("stage_changed".into()));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_nonfinite_duration() {
        assert!(
            command(CommandBody {
                generation: 1,
                command: "sleep".into(),
                direction: None,
                duration: Some(f32::NAN)
            })
            .is_err()
        )
    }
}
