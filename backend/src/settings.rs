use miette::IntoDiagnostic;
use secrecy::SecretString;

#[derive(serde::Deserialize, Clone, Debug)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub redis_url: SecretString,
}

pub fn get_settings() -> miette::Result<Settings> {
    let mut env = std::env::var("APP_ENVIRONMENT").unwrap_or_else(|_| "local".to_string());
    env.push_str(".toml");

    let mut dir = std::env::current_dir().into_diagnostic()?;
    dir.push("config");

    let settings = config::Config::builder()
        .add_source(config::File::from(dir.join("base.toml")))
        .add_source(config::File::from(dir.join(env)))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()
        .into_diagnostic()?;

    Ok(settings.try_deserialize().into_diagnostic()?)
}
