// SPDX-License-Identifier: MPL-2.0

//! Server configuration, read from a TOML file.

use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

/// Configuration file used when `ENTRYLENS_CONFIG` is not set.
pub const DEFAULT_PATH: &str = "/etc/entrylens.toml";

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Address and port of the HTTP server.
    pub listen: SocketAddr,
    /// Path of the SQLite database file.
    pub database: PathBuf,
    /// Name of the site, shown in the web interface.
    pub title: String,
    /// When the configuration page highlights a doorway's sensor.
    pub monitoring: Monitoring,
}

/// Limits of the sensor status; a doorway beyond any of them is highlighted.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Monitoring {
    /// Hours without a detected passage.
    pub no_passage_hours: u32,
    /// Hours without a sensor report.
    pub no_report_hours: u32,
    /// Lowest acceptable input voltage of the sensors, in volts.
    pub min_voltage: f64,
}

impl Default for Monitoring {
    fn default() -> Self {
        Self {
            no_passage_hours: 24,
            no_report_hours: 6,
            min_voltage: 4.5,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from(([0, 0, 0, 0], 8080)),
            database: PathBuf::from("/var/lib/entrylens/entrylens.db"),
            title: "Entrylens".to_owned(),
            monitoring: Monitoring::default(),
        }
    }
}

impl Config {
    /// Loads the configuration from `ENTRYLENS_CONFIG`, or from [`DEFAULT_PATH`]. A missing
    /// default file means all defaults; a missing file named by `ENTRYLENS_CONFIG` is an error.
    pub fn load() -> Result<Self, String> {
        match std::env::var("ENTRYLENS_CONFIG") {
            Ok(path) => Self::read(&path, true),
            Err(_) => Self::read(DEFAULT_PATH, false),
        }
    }

    /// Reads the configuration file `path`; if it is missing and not `explicit`ly named, the
    /// defaults.
    fn read(path: &str, explicit: bool) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text).map_err(|e| format!("{path}: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !explicit => {
                tracing::info!("{path} not found, using defaults");
                Ok(Self::default())
            }
            Err(e) => Err(format!("cannot read {path}: {e}")),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let config: Self = toml::from_str(text).map_err(|e| e.to_string())?;
        let m = &config.monitoring;
        if m.no_passage_hours == 0 || m.no_report_hours == 0 {
            return Err("[monitoring]: the hours must be at least 1".to_owned());
        }
        if !(m.min_voltage.is_finite() && m.min_voltage > 0.0) {
            return Err("[monitoring]: min_voltage must be a positive number of volts".to_owned());
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_defaults() {
        let config = Config::parse("").unwrap();
        assert_eq!(config.listen.port(), 8080);
        assert_eq!(
            config.database,
            PathBuf::from("/var/lib/entrylens/entrylens.db")
        );
    }

    #[test]
    fn keys_are_read() {
        let config = Config::parse(
            "listen = \"127.0.0.1:9000\"\ndatabase = \"x.db\"\ntitle = \"Nasi Nemci\"",
        )
        .unwrap();
        assert_eq!(config.listen, "127.0.0.1:9000".parse().unwrap());
        assert_eq!(config.database, PathBuf::from("x.db"));
        assert_eq!(config.title, "Nasi Nemci");
        assert_eq!(config.monitoring.no_passage_hours, 24);
    }

    #[test]
    fn monitoring_limits_are_read() {
        let config = Config::parse("[monitoring]\nno_report_hours = 2\nmin_voltage = 4.2").unwrap();
        assert_eq!(config.monitoring.no_passage_hours, 24);
        assert_eq!(config.monitoring.no_report_hours, 2);
        assert_eq!(config.monitoring.min_voltage, 4.2);
        assert!(Config::parse("[monitoring]\nno_signal = 1").is_err());
        assert!(Config::parse("[monitoring]\nno_report_hours = 0").is_err());
        assert!(Config::parse("[monitoring]\nmin_voltage = -1.0").is_err());
    }

    #[test]
    fn files_are_read() {
        let dir = std::env::temp_dir().join(format!("entrylens-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, text: &str| {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            path.to_str().unwrap().to_owned()
        };
        let good = file("good.toml", "title = \"Nasi Nemci\"");
        let bad = file("bad.toml", "titel = 1");
        let missing = dir.join("missing.toml").to_str().unwrap().to_owned();

        assert_eq!(Config::read(&good, true).unwrap().title, "Nasi Nemci");
        // A missing default file means the defaults; a missing named file is an error.
        assert_eq!(Config::read(&missing, false).unwrap().title, "Entrylens");
        assert!(
            Config::read(&missing, true)
                .unwrap_err()
                .starts_with("cannot read")
        );
        // Errors name the file.
        assert!(Config::read(&bad, true).unwrap_err().starts_with(&bad));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(Config::parse("lisen = \"127.0.0.1:9000\"").is_err());
    }
}
