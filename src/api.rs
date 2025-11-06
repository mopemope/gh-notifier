use crate::HistoryManager;
use actix_cors::Cors;
use actix_web::{App, HttpResponse, HttpServer, Result, middleware::Logger, web};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize)]
struct MarkAsReadRequest {
    notification_ids: Vec<String>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

// API endpoint to mark notifications as read
async fn mark_as_read(
    data: web::Data<Arc<Mutex<HistoryManager>>>,
    request: web::Json<MarkAsReadRequest>,
) -> Result<HttpResponse> {
    let history_manager = data.lock().unwrap();

    for notification_id in &request.notification_ids {
        if let Err(e) = history_manager.mark_as_read(notification_id) {
            tracing::error!(
                "Failed to mark notification {} as read: {}",
                notification_id,
                e
            );
            return Ok(HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: format!(
                    "Failed to mark notification {} as read: {}",
                    notification_id, e
                ),
            }));
        }
    }

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!(
            "Successfully marked {} notifications as read",
            request.notification_ids.len()
        ),
    }))
}

// API endpoint to mark all notifications as read
async fn mark_all_as_read(data: web::Data<Arc<Mutex<HistoryManager>>>) -> Result<HttpResponse> {
    let history_manager = data.lock().unwrap();

    if let Err(e) = history_manager.mark_all_as_read() {
        tracing::error!("Failed to mark all notifications as read: {}", e);
        return Ok(HttpResponse::InternalServerError().json(ApiResponse {
            success: false,
            message: format!("Failed to mark all notifications as read: {}", e),
        }));
    }

    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Successfully marked all notifications as read".to_string(),
    }))
}

// API endpoint to get all notifications
async fn get_notifications(data: web::Data<Arc<Mutex<HistoryManager>>>) -> Result<HttpResponse> {
    let history_manager = data.lock().unwrap();

    match history_manager.get_all_notifications() {
        Ok(notifications) => Ok(HttpResponse::Ok().json(notifications)),
        Err(e) => {
            tracing::error!("Failed to get notifications: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: format!("Failed to get notifications: {}", e),
            }))
        }
    }
}

// API endpoint to get unread notifications only
async fn get_unread_notifications(
    data: web::Data<Arc<Mutex<HistoryManager>>>,
) -> Result<HttpResponse> {
    let history_manager = data.lock().unwrap();

    match history_manager.get_unread_notifications() {
        Ok(notifications) => Ok(HttpResponse::Ok().json(notifications)),
        Err(e) => {
            tracing::error!("Failed to get unread notifications: {}", e);
            Ok(HttpResponse::InternalServerError().json(ApiResponse {
                success: false,
                message: format!("Failed to get unread notifications: {}", e),
            }))
        }
    }
}

pub struct ApiServer {
    history_manager: Arc<Mutex<HistoryManager>>,
    port: u16,
}

impl ApiServer {
    pub fn new(history_manager: HistoryManager, port: u16) -> Self {
        Self {
            history_manager: Arc::new(Mutex::new(history_manager)),
            port,
        }
    }

    pub async fn start(&self) -> std::io::Result<()> {
        let history_manager = self.history_manager.clone();
        let port = self.port;

        HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header();

            App::new()
                .app_data(web::Data::new(history_manager.clone()))
                .wrap(cors)
                .wrap(Logger::default())
                .route(
                    "/api/v1/notifications/mark-as-read",
                    web::post().to(mark_as_read),
                )
                .route(
                    "/api/v1/notifications/mark-all-as-read",
                    web::post().to(mark_all_as_read),
                )
                .route("/api/v1/notifications", web::get().to(get_notifications))
                .route(
                    "/api/v1/notifications/unread",
                    web::get().to(get_unread_notifications),
                )
        })
        .bind(("127.0.0.1", port))?
        .run()
        .await
    }
}
