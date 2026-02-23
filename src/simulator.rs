use std::collections::HashMap;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::config::PhoneModel;

#[derive(Debug, Deserialize)]
struct SimctlDeviceList {
    devices: HashMap<String, Vec<SimctlDevice>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SimctlDevice {
    name: String,
    udid: String,
    state: String,
    is_available: bool,
    device_type_identifier: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Simulator {
    pub name: String,
    pub udid: String,
    pub state: String,
    pub is_available: bool,
    pub device_type: String,
    pub runtime: String,
    pub phone_model: Option<PhoneModel>,
}

impl Simulator {
    pub fn is_booted(&self) -> bool {
        self.state == "Booted"
    }
}

/// Query all available simulators from simctl
pub fn list_simulators() -> Result<Vec<Simulator>> {
    let output = Command::new("xcrun")
        .args(["simctl", "list", "devices", "--json"])
        .output()
        .context("failed to execute xcrun simctl list devices")?;

    if !output.status.success() {
        bail!(
            "simctl list devices failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let json_str = String::from_utf8(output.stdout)
        .context("simctl output is not valid UTF-8")?;

    let device_list: SimctlDeviceList = serde_json::from_str(&json_str)
        .context("failed to parse simctl JSON output")?;

    let mut simulators = Vec::new();

    for (runtime, devices) in device_list.devices {
        for device in devices {
            if !device.is_available {
                continue;
            }

            let phone_model = detect_phone_model(&device.device_type_identifier);

            simulators.push(Simulator {
                name: device.name,
                udid: device.udid,
                state: device.state,
                is_available: device.is_available,
                device_type: device.device_type_identifier,
                runtime: runtime.clone(),
                phone_model,
            });
        }
    }

    // Sort by state (Booted first), then by name
    simulators.sort_by(|a, b| {
        let a_booted = a.is_booted();
        let b_booted = b.is_booted();
        match (a_booted, b_booted) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(simulators)
}

/// Find a simulator by name (exact or partial match) or UDID
pub fn find_simulator(query: &str) -> Result<Simulator> {
    let simulators = list_simulators()?;

    if simulators.is_empty() {
        bail!("no available simulators found");
    }

    // First, try exact UDID match
    if let Some(sim) = simulators.iter().find(|s| s.udid == query) {
        return Ok(sim.clone());
    }

    // Then, try exact name match
    if let Some(sim) = simulators.iter().find(|s| s.name == query) {
        return Ok(sim.clone());
    }

    // Then, try case-insensitive exact match
    let query_lower = query.to_lowercase();
    if let Some(sim) = simulators.iter().find(|s| s.name.to_lowercase() == query_lower) {
        return Ok(sim.clone());
    }

    // Then, try partial name match (contains)
    let matches: Vec<_> = simulators
        .iter()
        .filter(|s| s.name.to_lowercase().contains(&query_lower))
        .collect();

    match matches.len() {
        0 => {
            // No matches - provide helpful error with available simulators
            let booted: Vec<_> = simulators.iter().filter(|s| s.is_booted()).collect();
            let mut msg = format!("no simulator found matching '{}'", query);

            if !booted.is_empty() {
                msg.push_str("\n\nCurrently booted simulators:");
                for sim in &booted {
                    msg.push_str(&format!("\n  - {} ({})", sim.name, sim.udid));
                }
            } else {
                msg.push_str("\n\nNo simulators are currently booted.");
                msg.push_str("\n\nAvailable simulators (first 10):");
                for sim in simulators.iter().take(10) {
                    msg.push_str(&format!("\n  - {} [{}]", sim.name, sim.state));
                }
            }

            bail!("{}", msg);
        }
        1 => Ok(matches[0].clone()),
        _ => {
            // Multiple matches - ask user to be more specific
            let mut msg = format!(
                "multiple simulators match '{}', please be more specific:",
                query
            );
            for sim in &matches {
                let state_indicator = if sim.is_booted() { " (booted)" } else { "" };
                msg.push_str(&format!("\n  - {}{} [{}]", sim.name, state_indicator, sim.udid));
            }
            bail!("{}", msg);
        }
    }
}

/// Find all booted simulators
pub fn find_booted_simulators() -> Result<Vec<Simulator>> {
    let simulators = list_simulators()?;
    Ok(simulators.into_iter().filter(|s| s.is_booted()).collect())
}

/// Map device type identifier to PhoneModel
fn detect_phone_model(device_type: &str) -> Option<PhoneModel> {
    // device_type looks like: com.apple.CoreSimulator.SimDeviceType.iPhone-16-Pro
    let suffix = device_type.rsplit('.').next()?;

    match suffix {
        "iPhone-16-Pro" => Some(PhoneModel::Iphone16Pro),
        "iPhone-16-Pro-Max" => Some(PhoneModel::Iphone16ProMax),
        "iPhone-17-Pro" => Some(PhoneModel::Iphone17Pro),
        "iPhone-17-Pro-Max" => Some(PhoneModel::Iphone17ProMax),
        // Map older devices to closest model for reasonable defaults
        "iPhone-15-Pro" => Some(PhoneModel::Iphone16Pro),
        "iPhone-15-Pro-Max" => Some(PhoneModel::Iphone16ProMax),
        "iPhone-15" | "iPhone-15-Plus" => Some(PhoneModel::Iphone16Pro),
        "iPhone-14-Pro" => Some(PhoneModel::Iphone16Pro),
        "iPhone-14-Pro-Max" => Some(PhoneModel::Iphone16ProMax),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_phone_model() {
        assert_eq!(
            detect_phone_model("com.apple.CoreSimulator.SimDeviceType.iPhone-16-Pro"),
            Some(PhoneModel::Iphone16Pro)
        );
        assert_eq!(
            detect_phone_model("com.apple.CoreSimulator.SimDeviceType.iPhone-16-Pro-Max"),
            Some(PhoneModel::Iphone16ProMax)
        );
        assert_eq!(
            detect_phone_model("com.apple.CoreSimulator.SimDeviceType.iPhone-15-Pro"),
            Some(PhoneModel::Iphone16Pro)
        );
        assert_eq!(
            detect_phone_model("com.apple.CoreSimulator.SimDeviceType.Apple-Watch-Series-7-45mm"),
            None
        );
    }
}
