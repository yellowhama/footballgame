use crate::models::player::Position;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::OnceLock;

const CA_WEIGHTS_YAML: &str = include_str!("ca_weights_v0.yaml");
static CA_WEIGHTS: OnceLock<CAWeights> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
pub struct RangeSpec {
    pub min: u8,
    pub max: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttrCap {
    #[serde(default)]
    pub min: Option<u8>,
    #[serde(default)]
    pub max: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PositionGroup {
    pub weights: HashMap<String, u8>,
    #[serde(default)]
    pub caps: HashMap<String, AttrCap>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PositionRef {
    pub group: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CAWeightsFile {
    pub version: u8,
    pub attr_range: RangeSpec,
    pub weight_range: RangeSpec,
    pub attr_keys: Vec<String>,
    pub groups: HashMap<String, PositionGroup>,
    pub positions: HashMap<String, PositionRef>,
}

#[derive(Debug, Clone)]
pub struct CAWeights {
    pub version: u8,
    pub attr_range: RangeSpec,
    pub weight_range: RangeSpec,
    pub attr_keys: Vec<String>,
    pub groups: HashMap<String, PositionGroup>,
    pub positions: HashMap<Position, String>,
}

impl CAWeights {
    pub fn group_for_position(&self, position: Position) -> Option<&PositionGroup> {
        self.positions.get(&position).and_then(|group| self.groups.get(group))
    }

    pub fn attr_keys(&self) -> &[String] {
        &self.attr_keys
    }

    fn validate(&self) -> Result<(), String> {
        let attr_set: HashSet<&str> = self.attr_keys.iter().map(|k| k.as_str()).collect();
        if attr_set.is_empty() {
            return Err("attr_keys is empty".to_string());
        }

        for (group_name, group) in &self.groups {
            for key in &self.attr_keys {
                if !group.weights.contains_key(key) {
                    return Err(format!("group {} missing weight for {}", group_name, key));
                }
            }
            for key in group.weights.keys() {
                if !attr_set.contains(key.as_str()) {
                    return Err(format!("group {} has unknown weight key {}", group_name, key));
                }
                let w = group.weights[key];
                if w < self.weight_range.min || w > self.weight_range.max {
                    return Err(format!("group {} weight {} out of range", group_name, key));
                }
            }
            for key in group.caps.keys() {
                if !attr_set.contains(key.as_str()) {
                    return Err(format!("group {} has unknown cap key {}", group_name, key));
                }
            }
        }

        Ok(())
    }
}

impl TryFrom<CAWeightsFile> for CAWeights {
    type Error = String;

    fn try_from(file: CAWeightsFile) -> Result<Self, Self::Error> {
        let mut positions = HashMap::new();
        for (pos_key, group_ref) in &file.positions {
            let position = Position::from_str(pos_key)
                .map_err(|_| format!("invalid position key {}", pos_key))?;
            if !file.groups.contains_key(&group_ref.group) {
                return Err(format!("missing group {} for position {}", group_ref.group, pos_key));
            }
            positions.insert(position, group_ref.group.clone());
        }

        let weights = Self {
            version: file.version,
            attr_range: file.attr_range,
            weight_range: file.weight_range,
            attr_keys: file.attr_keys,
            groups: file.groups,
            positions,
        };
        weights.validate()?;
        Ok(weights)
    }
}

pub fn get_ca_weights() -> &'static CAWeights {
    CA_WEIGHTS.get_or_init(|| {
        let file: CAWeightsFile =
            serde_yaml::from_str(CA_WEIGHTS_YAML).expect("CA weights YAML invalid");
        CAWeights::try_from(file).expect("CA weights failed validation")
    })
}
