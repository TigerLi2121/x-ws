use crate::AppState;
use axum::extract::State;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;
use tracing::{error, info};
use uuid::Uuid;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("ws:{:?}", ws);
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let client_id = Uuid::new_v4();
    // 创建一个新的广播通道
    let (tx, mut rx) = broadcast::channel(1000);

    // 将新连接添加到共享状态
    {
        let mut clients = state.clients.lock().await;
        clients.insert(client_id, tx.clone());
        info!("新增连接用户:{} 当前总用户数:{}", client_id, clients.len());
    }


    // 处理接收到的消息
    let mut recv_task = tokio::spawn({
        let clients = state.clients.clone();
        async move {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(text) => {
                        info!("用户:{} 发送客户文本消息：{}", client_id, text);
                        let clients = clients.lock().await;
                        for (&ref id, tx) in clients.iter() {
                            if let Err(e) = tx.send(text.clone()) {
                                error!("Failed to send message to client {}: {}", id, e);
                            }
                        }
                    }
                    // Message::Binary(_) => {}
                    Message::Close(_) => {
                        info!("用户:{} 关闭连接", client_id);
                        break;
                    }
                    _ => info!("用户:{} 收到客户端消息：{:?}", client_id, msg),
                }
            }
        }
    });

    // 发送消息给当前客户端
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Err(e) = sender.send(Message::Text(msg)).await {
                error!("Failed to send message to client {}: {}", client_id, e);
                break;
            }
        }
    });

    // 等待任意一个任务完成
    tokio::select! {
        _ = (&mut recv_task) => send_task.abort(),
        _ = (&mut send_task) => recv_task.abort(),
    }

    // 连接关闭时，从共享状态中移除
    {
        let mut clients = state.clients.lock().await;
        clients.remove(&client_id);
        info!("Client disconnected: {}", client_id);
    }
}
