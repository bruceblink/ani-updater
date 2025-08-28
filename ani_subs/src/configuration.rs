use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::collections::HashMap;
use std::path::PathBuf;
use timer_tasker::task::TaskMeta;

#[derive(serde::Deserialize)]
pub struct Setting {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    // email_client
    pub email_client: EmailClientSettings,
    pub task_config: TaskConfig,
}

#[derive(serde::Deserialize)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    // 新的密钥配置项
    pub authorization_token: Secret<String>,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    /// 从环境变量或默认值初始化
    pub fn from_env(database: DatabaseSettings) -> Self {
        let host = std::env::var("DB_HOST").unwrap_or_else(|_| database.host.clone());
        let username = std::env::var("DB_USERNAME").unwrap_or_else(|_| database.username.clone());
        let password = Secret::new(
            std::env::var("POSTGRES_PASSWORD")
                .unwrap_or_else(|_| database.password.expose_secret().clone()),
        );
        let port = std::env::var("DB_PORT")
            .unwrap_or_else(|_| database.port.to_string())
            .parse()
            .unwrap();
        let database_name =
            std::env::var("DB_NAME").unwrap_or_else(|_| database.database_name.clone());
        let require_ssl = std::env::var("DB_REQUIRE_SSL")
            .ok()
            .map(|v| v == "true")
            .unwrap_or(database.require_ssl);

        DatabaseSettings {
            host,
            username,
            password,
            port,
            database_name,
            require_ssl,
        }
    }

    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.database_name)
    }
}

pub type TaskConfig = HashMap<String, Vec<TaskMeta>>;

/// ------------------------ 环境 ------------------------
pub enum Environment {
    Local,
    Production,
}

impl std::str::FromStr for Environment {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            _ => Ok(Environment::Production), // 默认生产环境
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Local => write!(f, "local"),
            Environment::Production => write!(f, "production"),
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or `production`.",
                other
            )),
        }
    }
}

/// ------------------------ 配置加载 ------------------------
pub fn get_configuration(
    config_dir: Option<PathBuf>, // 可选覆盖目录
) -> Result<Setting, config::ConfigError> {
    // 配置目录默认使用 crate 根目录下的 configuration
    let config_directory = config_dir
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("configuration"));

    // 环境默认 local
    let env: Environment = std::env::var("APP_ENV")
        .unwrap_or_else(|_| "production".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");
    let env_filename = format!("{}.yaml", env);

    // 构建配置
    let settings = config::Config::builder()
        .add_source(config::File::from(config_directory.join("base.yaml")))
        .add_source(config::File::from(config_directory.join(env_filename)))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Setting>()
}

mod tests {
    #[test]
    fn test_environment_from_str() {
        use super::Environment;
        let env: Environment = "local".parse().unwrap();
        assert!(matches!(env, Environment::Local));

        let env: Environment = "production".parse().unwrap();
        assert!(matches!(env, Environment::Production));

        let env: Environment = "unknown".parse().unwrap();
        assert!(matches!(env, Environment::Production)); // 默认生产环境

        unsafe {
            std::env::set_var("APP_ENV", "local");
        }
        let environment: Environment = std::env::var("APP_ENV")
            .unwrap_or_else(|_| "local".into())
            .try_into()
            .expect("Failed to parse APP_ENVIRONMENT.");
        assert!(matches!(environment, Environment::Local)); //

        unsafe {
            std::env::set_var("APP_ENV", "production");
        }
        let environment: Environment = std::env::var("APP_ENV")
            .unwrap_or_else(|_| "local".into())
            .try_into()
            .expect("Failed to parse APP_ENVIRONMENT.");
        assert!(matches!(environment, Environment::Production)); // 
    }
}
