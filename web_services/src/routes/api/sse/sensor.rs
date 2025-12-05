use crate::common::AppState;
use actix_web::http::header;
use actix_web::{Error, HttpResponse, get, web};
use chrono_tz::Asia::Shanghai;
use common::po::ApiResult;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize, Clone)]
#[allow(dead_code)]
pub struct SensorData {
    timestamp: String,
    temperature: i16,
    humidity: i16,
}

#[get("/sse/sensor")]
async fn sse_sensor(app_state: web::Data<AppState>) -> ApiResult {
    // 创建事件流
    let stream = futures::stream::unfold((), move |_| {
        let state = app_state.clone();
        async move {
            // 模拟数据生成间隔
            tokio::time::sleep(Duration::from_secs(2)).await;

            // 生成模拟数据
            let data = SensorData {
                timestamp: chrono::Utc::now()
                    .with_timezone(&Shanghai)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
                temperature: (rand::random::<f64>() * 10.0 + 20.0) as i16, // 20-30°C
                humidity: (rand::random::<f64>() * 40.0 + 30.0) as i16,    // 30-70%
            };
            // 保存历史数据
            state.add_data(data.clone()).await;
            // 格式化为 SSE 格式
            let event_data = format!("data: {}\n\n", serde_json::to_string(&data).unwrap());

            Some((Ok::<_, Error>(web::Bytes::from(event_data)), ()))
        }
    });

    Ok(HttpResponse::Ok()
        .insert_header(header::ContentType(mime::TEXT_EVENT_STREAM))
        .insert_header(header::CacheControl(vec![header::CacheDirective::NoCache]))
        .streaming(stream))
}

#[get("/sensor/history")]
async fn get_sensor_history(app_state: web::Data<AppState>) -> ApiResult {
    let history = app_state.get_history().await;
    Ok(HttpResponse::Ok().json(history))
}
