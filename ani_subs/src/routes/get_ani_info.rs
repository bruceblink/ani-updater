use crate::dao::postgresql::ani_info_table::get_ani_info_by_id;
use crate::dao::postgresql::ani_info_table::list_all_ani_info;
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

pub async fn get_ani_info(path: web::Path<(i64,)>, pool: web::Data<PgPool>) -> HttpResponse {
    let ani_id = path.into_inner().0; // 从元组里取值
    match get_ani_info_by_id(ani_id, &pool).await {
        Ok(Some(ani)) => HttpResponse::Ok().json(ani), // 查到 → 返回 JSON
        Ok(None) => HttpResponse::NotFound().finish(), // 没查到 → 返回 404
        Err(e) => {
            eprintln!("数据库查询错误: {:?}", e);
            HttpResponse::InternalServerError().finish() // SQL 错误 → 500
        }
    }
}

pub async fn get_ani_info_list(pool: web::Data<PgPool>) -> HttpResponse {
    match list_all_ani_info("ani_id".to_string(), &pool).await {
        Ok(ani_list) => HttpResponse::Ok().json(ani_list), // 查到 → 返回 JSON
        Err(e) => {
            eprintln!("数据库查询错误: {:?}", e);
            HttpResponse::InternalServerError().finish() // SQL 错误 → 500
        }
    }
}
