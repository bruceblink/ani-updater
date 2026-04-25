use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, web};
use common::api::ApiError;
use common::utils::JwtClaims;

mod create;
mod delete;
mod get;
mod toggle;
mod update;

fn has_admin_access(claims: &JwtClaims, permissions: &[String]) -> bool {
    claims.roles.iter().any(|r| r == "admin") || permissions.iter().any(|p| p == "admin:all")
}

pub async fn ensure_admin_access(
    req: &HttpRequest,
    app_state: &web::Data<AppState>,
) -> Result<(), ApiError> {
    let claims = req
        .extensions()
        .get::<JwtClaims>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized("未授权".into()))?;

    let permissions: Vec<String> = sqlx::query_scalar(
        r#"
            SELECT DISTINCT p.name
            FROM permissions p
            JOIN role_permissions rp ON rp.permission_id = p.id
            JOIN user_roles ur ON ur.role_id = rp.role_id
            WHERE ur.user_id = $1
        "#,
    )
    .bind(claims.uid)
    .fetch_all(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询用户权限失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?;

    if !has_admin_access(&claims, &permissions) {
        return Err(ApiError::Forbidden("需要管理员权限".into()));
    }

    Ok(())
}

pub use create::*;
pub use delete::*;
pub use get::*;
pub use toggle::*;
pub use update::*;

#[cfg(test)]
mod tests {
    use super::has_admin_access;
    use common::utils::JwtClaims;

    fn build_claims(roles: Vec<&str>) -> JwtClaims {
        JwtClaims {
            sub: "u1".into(),
            exp: 0,
            iat: 0,
            uid: 1,
            email: None,
            avatar: None,
            roles: roles.into_iter().map(str::to_string).collect(),
            ver: 1,
        }
    }

    #[test]
    fn has_admin_access_accepts_admin_role_or_admin_permission() {
        let claims_with_admin_role = build_claims(vec!["admin"]);
        assert!(has_admin_access(&claims_with_admin_role, &[]));

        let claims_without_admin_role = build_claims(vec!["user"]);
        assert!(has_admin_access(
            &claims_without_admin_role,
            &["admin:all".to_string()]
        ));

        assert!(!has_admin_access(&claims_without_admin_role, &[]));
    }
}
