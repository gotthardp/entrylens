// SPDX-License-Identifier: MPL-2.0

//! The status of the doorways' sensors, against the limits of the `[monitoring]` settings.

use serde::Serialize;
use utoipa::ToSchema;

use crate::config::Monitoring;
use crate::db::Doorway;

/// A doorway with the status of its sensor at the time of the request.
#[derive(Debug, Serialize, ToSchema)]
pub struct DoorwayWithStatus {
    #[serde(flatten)]
    pub doorway: Doorway,
    /// No passage within the configured period, or none yet.
    pub no_passage: bool,
    /// No sensor report within the configured period, or none yet.
    pub no_report: bool,
    /// The last reported input voltage is below the lowest acceptable one.
    pub low_voltage: bool,
}

impl DoorwayWithStatus {
    /// The doorway's sensor status at `now` (Unix time).
    pub fn new(doorway: Doorway, limits: &Monitoring, now: i64) -> Self {
        let older =
            |t: Option<i64>, hours: u32| t.is_none_or(|t| now - t > i64::from(hours) * 3600);
        Self {
            no_passage: older(doorway.last_passage, limits.no_passage_hours),
            no_report: older(doorway.last_report, limits.no_report_hours),
            low_voltage: doorway
                .voltage_mv
                .is_some_and(|mv| (mv as f64) < limits.min_voltage * 1000.0),
            doorway,
        }
    }
}
