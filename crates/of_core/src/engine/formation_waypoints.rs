//! Formation-specific waypoint data
//!
//! Provides pre-defined position waypoints for all supported formations.
//! Coordinates are normalized (0-1) where:
//! - X: 0 = left touchline, 1 = right touchline
//! - Y: 0 = own goal line, 1 = opponent goal line

use super::positioning::{PositionKey, PositionWaypoints};
use std::collections::HashMap;

/// Get waypoints for a specific formation
pub fn get_formation_waypoints(formation: &str) -> HashMap<PositionKey, PositionWaypoints> {
    match formation {
        "4-4-2" | "442" => get_442_waypoints(),
        "4-3-3" | "433" => get_433_waypoints(),
        "4-2-3-1" | "4231" => get_4231_waypoints(),
        "3-5-2" | "352" => get_352_waypoints(),
        "4-4-2-diamond" | "442d" => get_442_diamond_waypoints(),
        "4-1-4-1" | "4141" => get_4141_waypoints(),
        "4-4-1-1" | "4411" => get_4411_waypoints(),
        "4-3-2-1" | "4321" => get_4321_waypoints(),
        "3-4-3" | "343" => get_343_waypoints(),
        "5-3-2" | "532" => get_532_waypoints(),
        "4-3-1-2" | "4312" => get_4312_waypoints(),
        "4-2-2-2" | "4222" => get_4222_waypoints(),
        "5-4-1" | "541" => get_541_waypoints(),
        "4-5-1" | "451" => get_451_waypoints(),
        "3-4-2-1" | "3421" => get_3421_waypoints(),
        "3-4-1-2" | "3412" => get_3412_waypoints(),
        _ => panic!("UNSUPPORTED_FORMATION: get_formation_waypoints({formation})"),
    }
}

/// 4-4-2 Formation
fn get_442_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Midfield
    map.insert(PositionKey::LM, PositionWaypoints::from_base((0.15, 0.50), 0.12, 0.10));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.35, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.65, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RM, PositionWaypoints::from_base((0.85, 0.50), 0.12, 0.10));

    // Attack
    map.insert(PositionKey::LF, PositionWaypoints::from_base((0.35, 0.78), 0.10, 0.15));
    map.insert(PositionKey::RF, PositionWaypoints::from_base((0.65, 0.78), 0.10, 0.15));

    map
}

/// 4-3-3 Formation
fn get_433_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Midfield (3)
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.30, 0.45), 0.12, 0.12));
    map.insert(PositionKey::CM, PositionWaypoints::from_base((0.50, 0.42), 0.10, 0.15));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.70, 0.45), 0.12, 0.12));

    // Attack (3)
    map.insert(PositionKey::LW, PositionWaypoints::from_base((0.12, 0.75), 0.12, 0.08));
    map.insert(PositionKey::ST, PositionWaypoints::from_base((0.50, 0.82), 0.10, 0.15));
    map.insert(PositionKey::RW, PositionWaypoints::from_base((0.88, 0.75), 0.12, 0.08));

    map
}

/// 4-2-3-1 Formation
fn get_4231_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Defensive Midfield (2)
    map.insert(PositionKey::LDM, PositionWaypoints::from_base((0.35, 0.38), 0.08, 0.12));
    map.insert(PositionKey::RDM, PositionWaypoints::from_base((0.65, 0.38), 0.08, 0.12));

    // Attacking Midfield (3)
    map.insert(PositionKey::LAM, PositionWaypoints::from_base((0.20, 0.62), 0.12, 0.10));
    map.insert(PositionKey::CAM, PositionWaypoints::from_base((0.50, 0.60), 0.12, 0.15));
    map.insert(PositionKey::RAM, PositionWaypoints::from_base((0.80, 0.62), 0.12, 0.10));

    // Attack
    map.insert(PositionKey::ST, PositionWaypoints::from_base((0.50, 0.82), 0.10, 0.15));

    map
}

/// 3-5-2 Formation
fn get_352_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense (3)
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.25, 0.20), 0.06, 0.10));
    map.insert(PositionKey::CB, PositionWaypoints::from_base((0.50, 0.18), 0.05, 0.12));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.75, 0.20), 0.06, 0.10));

    // Midfield (5)
    map.insert(PositionKey::LWB, PositionWaypoints::from_base((0.10, 0.45), 0.15, 0.08));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.30, 0.42), 0.10, 0.10));
    map.insert(PositionKey::CM, PositionWaypoints::from_base((0.50, 0.40), 0.08, 0.12));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.70, 0.42), 0.10, 0.10));
    map.insert(PositionKey::RWB, PositionWaypoints::from_base((0.90, 0.45), 0.15, 0.08));

    // Attack (2)
    map.insert(PositionKey::LF, PositionWaypoints::from_base((0.35, 0.78), 0.10, 0.15));
    map.insert(PositionKey::RF, PositionWaypoints::from_base((0.65, 0.78), 0.10, 0.15));

    map
}

/// 4-4-2 Diamond Formation
fn get_442_diamond_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Diamond Midfield
    map.insert(PositionKey::CDM, PositionWaypoints::from_base((0.50, 0.35), 0.08, 0.12));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.30, 0.48), 0.10, 0.10));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.70, 0.48), 0.10, 0.10));
    map.insert(PositionKey::CAM, PositionWaypoints::from_base((0.50, 0.60), 0.12, 0.15));

    // Attack
    map.insert(PositionKey::LF, PositionWaypoints::from_base((0.35, 0.78), 0.10, 0.15));
    map.insert(PositionKey::RF, PositionWaypoints::from_base((0.65, 0.78), 0.10, 0.15));

    map
}

/// 4-1-4-1 Formation
fn get_4141_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Defensive Midfield
    map.insert(PositionKey::CDM, PositionWaypoints::from_base((0.50, 0.35), 0.08, 0.15));

    // Midfield (4)
    map.insert(PositionKey::LM, PositionWaypoints::from_base((0.15, 0.55), 0.12, 0.10));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.35, 0.52), 0.10, 0.12));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.65, 0.52), 0.10, 0.12));
    map.insert(PositionKey::RM, PositionWaypoints::from_base((0.85, 0.55), 0.12, 0.10));

    // Attack
    map.insert(PositionKey::ST, PositionWaypoints::from_base((0.50, 0.82), 0.10, 0.15));

    map
}

/// 4-4-1-1 Formation
fn get_4411_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Midfield (4)
    map.insert(PositionKey::LM, PositionWaypoints::from_base((0.15, 0.48), 0.12, 0.10));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.35, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.65, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RM, PositionWaypoints::from_base((0.85, 0.48), 0.12, 0.10));

    // Support Striker
    map.insert(PositionKey::CF, PositionWaypoints::from_base((0.50, 0.68), 0.12, 0.15));

    // Striker
    map.insert(PositionKey::ST, PositionWaypoints::from_base((0.50, 0.82), 0.10, 0.15));

    map
}

/// 4-3-2-1 Formation
fn get_4321_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Midfield (3)
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.30, 0.45), 0.10, 0.12));
    map.insert(PositionKey::CM, PositionWaypoints::from_base((0.50, 0.42), 0.08, 0.15));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.70, 0.45), 0.10, 0.12));

    // Supporting Forwards (2)
    map.insert(PositionKey::LF, PositionWaypoints::from_base((0.40, 0.68), 0.12, 0.12));
    map.insert(PositionKey::RF, PositionWaypoints::from_base((0.60, 0.68), 0.12, 0.12));

    // Striker
    map.insert(PositionKey::ST, PositionWaypoints::from_base((0.50, 0.82), 0.10, 0.15));

    map
}

/// 3-4-3 Formation
fn get_343_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense (3)
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.25, 0.20), 0.06, 0.10));
    map.insert(PositionKey::CB, PositionWaypoints::from_base((0.50, 0.18), 0.05, 0.12));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.75, 0.20), 0.06, 0.10));

    // Midfield (4)
    map.insert(PositionKey::LM, PositionWaypoints::from_base((0.15, 0.48), 0.15, 0.10));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.35, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.65, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RM, PositionWaypoints::from_base((0.85, 0.48), 0.15, 0.10));

    // Attack (3)
    map.insert(PositionKey::LW, PositionWaypoints::from_base((0.15, 0.75), 0.12, 0.08));
    map.insert(PositionKey::ST, PositionWaypoints::from_base((0.50, 0.82), 0.10, 0.15));
    map.insert(PositionKey::RW, PositionWaypoints::from_base((0.85, 0.75), 0.12, 0.08));

    map
}

/// 5-3-2 Formation
fn get_532_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense (5)
    map.insert(PositionKey::LWB, PositionWaypoints::from_base((0.10, 0.30), 0.12, 0.08));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.28, 0.20), 0.06, 0.08));
    map.insert(PositionKey::CB, PositionWaypoints::from_base((0.50, 0.18), 0.05, 0.10));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.72, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RWB, PositionWaypoints::from_base((0.90, 0.30), 0.12, 0.08));

    // Midfield (3)
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.30, 0.45), 0.10, 0.12));
    map.insert(PositionKey::CM, PositionWaypoints::from_base((0.50, 0.42), 0.08, 0.15));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.70, 0.45), 0.10, 0.12));

    // Attack (2)
    map.insert(PositionKey::LF, PositionWaypoints::from_base((0.35, 0.78), 0.10, 0.15));
    map.insert(PositionKey::RF, PositionWaypoints::from_base((0.65, 0.78), 0.10, 0.15));

    map
}

/// 4-3-1-2 Formation
fn get_4312_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Midfield (3)
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.30, 0.42), 0.10, 0.12));
    map.insert(PositionKey::CM, PositionWaypoints::from_base((0.50, 0.38), 0.08, 0.15));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.70, 0.42), 0.10, 0.12));

    // Attacking Midfield
    map.insert(PositionKey::CAM, PositionWaypoints::from_base((0.50, 0.58), 0.12, 0.15));

    // Attack (2)
    map.insert(PositionKey::LF, PositionWaypoints::from_base((0.35, 0.78), 0.10, 0.15));
    map.insert(PositionKey::RF, PositionWaypoints::from_base((0.65, 0.78), 0.10, 0.15));

    map
}

/// 4-2-2-2 Formation
fn get_4222_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Defensive Midfield (2)
    map.insert(PositionKey::LDM, PositionWaypoints::from_base((0.35, 0.38), 0.08, 0.12));
    map.insert(PositionKey::RDM, PositionWaypoints::from_base((0.65, 0.38), 0.08, 0.12));

    // Attacking Midfield (2)
    map.insert(PositionKey::LAM, PositionWaypoints::from_base((0.30, 0.58), 0.12, 0.12));
    map.insert(PositionKey::RAM, PositionWaypoints::from_base((0.70, 0.58), 0.12, 0.12));

    // Attack (2)
    map.insert(PositionKey::LF, PositionWaypoints::from_base((0.35, 0.78), 0.10, 0.15));
    map.insert(PositionKey::RF, PositionWaypoints::from_base((0.65, 0.78), 0.10, 0.15));

    map
}

/// 5-4-1 Formation
fn get_541_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense (5)
    map.insert(PositionKey::LWB, PositionWaypoints::from_base((0.10, 0.28), 0.10, 0.08));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.28, 0.20), 0.06, 0.08));
    map.insert(PositionKey::CB, PositionWaypoints::from_base((0.50, 0.18), 0.05, 0.10));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.72, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RWB, PositionWaypoints::from_base((0.90, 0.28), 0.10, 0.08));

    // Midfield (4)
    map.insert(PositionKey::LM, PositionWaypoints::from_base((0.15, 0.48), 0.12, 0.10));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.35, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.65, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RM, PositionWaypoints::from_base((0.85, 0.48), 0.12, 0.10));

    // Attack
    map.insert(PositionKey::ST, PositionWaypoints::from_base((0.50, 0.80), 0.10, 0.15));

    map
}

/// 4-5-1 Formation
fn get_451_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense
    map.insert(PositionKey::LB, PositionWaypoints::from_base((0.15, 0.25), 0.08, 0.10));
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.35, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.65, 0.20), 0.06, 0.08));
    map.insert(PositionKey::RB, PositionWaypoints::from_base((0.85, 0.25), 0.08, 0.10));

    // Midfield (5)
    map.insert(PositionKey::LM, PositionWaypoints::from_base((0.12, 0.50), 0.12, 0.08));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.30, 0.45), 0.10, 0.10));
    map.insert(PositionKey::CM, PositionWaypoints::from_base((0.50, 0.42), 0.08, 0.12));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.70, 0.45), 0.10, 0.10));
    map.insert(PositionKey::RM, PositionWaypoints::from_base((0.88, 0.50), 0.12, 0.08));

    // Attack
    map.insert(PositionKey::ST, PositionWaypoints::from_base((0.50, 0.82), 0.10, 0.15));

    map
}

/// 3-4-1-2 Formation
fn get_3412_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense (3)
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.25, 0.20), 0.06, 0.10));
    map.insert(PositionKey::CB, PositionWaypoints::from_base((0.50, 0.18), 0.05, 0.12));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.75, 0.20), 0.06, 0.10));

    // Midfield (4)
    map.insert(PositionKey::LM, PositionWaypoints::from_base((0.12, 0.45), 0.15, 0.08));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.35, 0.42), 0.10, 0.10));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.65, 0.42), 0.10, 0.10));
    map.insert(PositionKey::RM, PositionWaypoints::from_base((0.88, 0.45), 0.15, 0.08));

    // Attacking Midfield
    map.insert(PositionKey::CAM, PositionWaypoints::from_base((0.50, 0.60), 0.12, 0.15));

    // Attack (2)
    map.insert(PositionKey::LF, PositionWaypoints::from_base((0.35, 0.78), 0.10, 0.15));
    map.insert(PositionKey::RF, PositionWaypoints::from_base((0.65, 0.78), 0.10, 0.15));

    map
}

/// 3-4-2-1 Formation
fn get_3421_waypoints() -> HashMap<PositionKey, PositionWaypoints> {
    let mut map = HashMap::new();

    // Goalkeeper
    map.insert(PositionKey::GK, PositionWaypoints::from_base((0.5, 0.04), 0.02, 0.08));

    // Defense (3)
    map.insert(PositionKey::LCB, PositionWaypoints::from_base((0.25, 0.20), 0.06, 0.10));
    map.insert(PositionKey::CB, PositionWaypoints::from_base((0.50, 0.18), 0.05, 0.12));
    map.insert(PositionKey::RCB, PositionWaypoints::from_base((0.75, 0.20), 0.06, 0.10));

    // Midfield (4)
    map.insert(PositionKey::LM, PositionWaypoints::from_base((0.15, 0.48), 0.15, 0.10));
    map.insert(PositionKey::LCM, PositionWaypoints::from_base((0.40, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RCM, PositionWaypoints::from_base((0.60, 0.45), 0.10, 0.12));
    map.insert(PositionKey::RM, PositionWaypoints::from_base((0.85, 0.48), 0.15, 0.10));

    // Attacking Midfield (2)
    map.insert(PositionKey::LAM, PositionWaypoints::from_base((0.35, 0.65), 0.12, 0.12));
    map.insert(PositionKey::RAM, PositionWaypoints::from_base((0.65, 0.65), 0.12, 0.12));

    // Striker
    map.insert(PositionKey::ST, PositionWaypoints::from_base((0.50, 0.82), 0.10, 0.15));

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_formations_have_11_players() {
        let formations = vec![
            // FIX01 allowlist (14)
            "4-4-2",
            "4-3-3",
            "4-2-3-1",
            "4-1-4-1",
            "4-4-1-1",
            "4-3-2-1",
            "4-2-2-2",
            "4-5-1",
            "3-5-2",
            "3-4-3",
            "3-4-2-1",
            "3-4-1-2",
            "5-3-2",
            "5-4-1",
        ];

        for formation in formations {
            let waypoints = get_formation_waypoints(formation);
            assert_eq!(
                waypoints.len(),
                11,
                "Formation {} should have 11 positions, got {}",
                formation,
                waypoints.len()
            );
        }
    }

    #[test]
    fn test_442_has_correct_positions() {
        let waypoints = get_formation_waypoints("4-4-2");

        assert!(waypoints.contains_key(&PositionKey::GK));
        assert!(waypoints.contains_key(&PositionKey::LB));
        assert!(waypoints.contains_key(&PositionKey::RB));
        assert!(waypoints.contains_key(&PositionKey::LCB));
        assert!(waypoints.contains_key(&PositionKey::RCB));
        assert!(waypoints.contains_key(&PositionKey::LM));
        assert!(waypoints.contains_key(&PositionKey::RM));
        assert!(waypoints.contains_key(&PositionKey::LF));
        assert!(waypoints.contains_key(&PositionKey::RF));
    }

    #[test]
    fn test_433_has_wingers() {
        let waypoints = get_formation_waypoints("4-3-3");

        assert!(waypoints.contains_key(&PositionKey::LW));
        assert!(waypoints.contains_key(&PositionKey::RW));
        assert!(waypoints.contains_key(&PositionKey::ST));
    }

    #[test]
    fn test_positions_are_valid() {
        let waypoints = get_formation_waypoints("4-4-2");

        for (key, wp) in &waypoints {
            // Check all coordinates are in valid range
            assert!(
                wp.base.0 >= 0.0 && wp.base.0 <= 1.0,
                "{:?} base x out of range: {}",
                key,
                wp.base.0
            );
            assert!(
                wp.base.1 >= 0.0 && wp.base.1 <= 1.0,
                "{:?} base y out of range: {}",
                key,
                wp.base.1
            );
        }
    }

    #[test]
    #[test]
    #[should_panic]
    fn test_unknown_formation_panics() {
        let _ = get_formation_waypoints("unknown");
    }

    #[test]
    fn test_goalkeeper_position() {
        let formations = vec!["4-4-2", "4-3-3", "3-5-2"];

        for formation in formations {
            let waypoints = get_formation_waypoints(formation);
            let gk = waypoints.get(&PositionKey::GK).unwrap();

            // GK should be near goal line
            assert!(gk.base.1 < 0.1, "{} GK should be near goal line: {}", formation, gk.base.1);
            // GK should be central
            assert!(
                (gk.base.0 - 0.5).abs() < 0.1,
                "{} GK should be central: {}",
                formation,
                gk.base.0
            );
        }
    }
}
