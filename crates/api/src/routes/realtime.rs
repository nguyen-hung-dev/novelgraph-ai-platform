use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
    routing::get,
    Router,
};
use tokio::sync::broadcast;

use crate::{AppState, ProjectRealtimeEvent};

pub(crate) fn router() -> Router<AppState> {
    Router::new().route(
        "/api/projects/{project_id}/realtime",
        get(project_realtime_ws),
    )
}

async fn project_realtime_ws(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| project_realtime_socket(socket, state, project_id))
}

async fn project_realtime_socket(mut socket: WebSocket, state: AppState, project_id: String) {
    let mut receiver = state.realtime_tx.subscribe();
    let connected = ProjectRealtimeEvent {
        project_id: project_id.clone(),
        event_type: "connected".to_string(),
        job_id: None,
        chapter_id: None,
        detail: "project realtime socket connected".to_string(),
    };

    if let Ok(payload) = serde_json::to_string(&connected) {
        if socket.send(Message::Text(payload.into())).await.is_err() {
            return;
        }
    }

    loop {
        match receiver.recv().await {
            Ok(event) if event.project_id == project_id => {
                let payload = match serde_json::to_string(&event) {
                    Ok(payload) => payload,
                    Err(_) => continue,
                };

                if socket.send(Message::Text(payload.into())).await.is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(broadcast::error::RecvError::Lagged(_)) => {
                let lagged = ProjectRealtimeEvent {
                    project_id: project_id.clone(),
                    event_type: "resync_required".to_string(),
                    job_id: None,
                    chapter_id: None,
                    detail: "client lagged behind realtime event stream".to_string(),
                };
                if let Ok(payload) = serde_json::to_string(&lagged) {
                    if socket.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}
