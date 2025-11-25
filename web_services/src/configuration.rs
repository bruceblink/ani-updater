use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Setting {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    // email_client
    pub email_client: EmailClientSettings,
    // token
    pub token: TokenConfig,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    // 新的密钥配置项
    pub authorization_token: Secret<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
    pub max_connections: u32, // 数据库连接池最大连接数
}

impl DatabaseSettings {
    /// 从环境变量或默认值初始化
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(host) = std::env::var("DB_HOST") {
            self.host = host;
        }

        if let Ok(username) = std::env::var("DB_USERNAME") {
            self.username = username;
        }

        if let Ok(password) = std::env::var("POSTGRES_PASSWORD") {
            self.password = Secret::new(password);
        }

        if let Ok(port) = std::env::var("DB_PORT")
            && let Ok(parsed_port) = port.parse()
        {
            self.port = parsed_port;
            // 可以添加日志记录解析失败的情况
        }

        if let Ok(database_name) = std::env::var("DB_NAME") {
            self.database_name = database_name;
        }

        if let Ok(require_ssl) = std::env::var("DB_REQUIRE_SSL") {
            self.require_ssl = require_ssl == "true";
        }

        if let Ok(max_connections) = std::env::var("DB_MAX_CONNS_NUM")
            && let Ok(max_conns) = max_connections.parse()
        {
            self.max_connections = max_conns;
        }

        self
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

pub type TokenConfig = HashMap<String, i16>;

/// ------------------------ 环境 ------------------------
#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }

    pub fn config_filename(&self) -> String {
        format!("{}.yaml", self.as_str())
    }
}

impl std::str::FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            other => Err(format!(
                "'{}' is not a supported environment. Use either 'local' or 'production'.",
                other
            )),
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// ------------------------ 配置加载 ------------------------
pub fn get_configuration(config_dir: Option<PathBuf>) -> Result<Setting, config::ConfigError> {
    let config_directory = config_dir
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("configuration"));

    let environment = get_environment()?;

    let settings = build_config(config_directory, &environment)?;

    // 应用环境变量覆盖
    let mut setting: Setting = settings.try_deserialize()?;
    setting.database = setting.database.with_env_overrides();

    Ok(setting)
}

/// 获取当前环境
fn get_environment() -> Result<Environment, config::ConfigError> {
    let env_str = std::env::var("APP_ENV").unwrap_or_else(|_| "production".into());

    env_str
        .parse()
        .map_err(|e: String| config::ConfigError::Message(e))
}

/// 构建配置
fn build_config(
    config_directory: PathBuf,
    environment: &Environment,
) -> Result<config::Config, config::ConfigError> {
    config::Config::builder()
        .add_source(config::File::from(config_directory.join("base.yaml")))
        .add_source(config::File::from(
            config_directory.join(environment.config_filename()),
        ))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_environment_from_str() {
        assert_eq!("local".parse(), Ok(Environment::Local));
        assert_eq!("production".parse(), Ok(Environment::Production));

        let result: Result<Environment, _> = "unknown".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(Environment::Local.to_string(), "local");
        assert_eq!(Environment::Production.to_string(), "production");
    }

    #[test]
    fn test_database_settings_env_overrides() {
        let original = DatabaseSettings {
            host: "localhost".to_string(),
            username: "test_user".to_string(),
            password: Secret::new("test_pass".to_string()),
            port: 5432,
            database_name: "test_db".to_string(),
            require_ssl: false,
            max_connections: 16,
        };

        // 设置环境变量
        unsafe {
            env::set_var("DB_HOST", "override_host");
            env::set_var("DB_USERNAME", "override_user");
            env::set_var("DB_MAX_CONNS_NUM", "override_max_conns");
        }

        let overridden = original.clone().with_env_overrides();

        assert_eq!(overridden.host, "override_host");
        assert_eq!(overridden.username, "override_user");
        // 其他字段应该保持不变
        assert_eq!(overridden.port, original.port);
        assert_eq!(overridden.database_name, original.database_name);
        assert_eq!(overridden.max_connections, original.max_connections);

        // 清理环境变量
        unsafe {
            env::remove_var("DB_HOST");
            env::remove_var("DB_USERNAME");
            env::remove_var("DB_MAX_CONNS_NUM");
        }
    }

    #[test]
    fn test_token_config() {
        let config = get_configuration(Some(PathBuf::from("../configuration"))).unwrap();
        assert_eq!(config.token.len(), 2);
        assert_eq!(config.token["access_token"], 20);
        assert_eq!(config.token["refresh_token"], 15);
    }
}
