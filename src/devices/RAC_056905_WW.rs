use serde_json::json;
use std::collections::HashMap;

use crate::device::{Field, HADevice};

#[allow(non_camel_case_types)]
#[derive(Clone)]
enum RAC_056905_WW_Fields {
    CurrentTemperature,
    Power,
    Mode,
    FanMode,
    Temperature,
    VerticalSwingMode,
    SwingMode,
}

impl RAC_056905_WW_Fields {
    fn from_id(id: u16) -> Option<Self> {
        match id {
            0x1fd => Some(Self::CurrentTemperature),
            0x1f7 => Some(Self::Power),
            0x1f9 => Some(Self::Mode),
            0x1fa => Some(Self::FanMode),
            0x1fe => Some(Self::Temperature),
            0x321 => Some(Self::VerticalSwingMode),
            0x322 => Some(Self::SwingMode),
            _ => None,
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "current_temperature" => Some(Self::CurrentTemperature),
            "power" => Some(Self::Power),
            "mode" => Some(Self::Mode),
            "fan_mode" => Some(Self::FanMode),
            "temperature" => Some(Self::Temperature),
            "vertical_swing_mode" => Some(Self::VerticalSwingMode),
            "swing_mode" => Some(Self::SwingMode),
            _ => None,
        }
    }
}

impl Field for RAC_056905_WW_Fields {
    fn id(&self) -> u16 {
        match self {
            Self::CurrentTemperature => 0x1fd,
            Self::Power => 0x1f7,
            Self::Mode => 0x1f9,
            Self::FanMode => 0x1fa,
            Self::Temperature => 0x1fe,
            Self::VerticalSwingMode => 0x321,
            Self::SwingMode => 0x322,
        }
    }

    fn name(&self) -> String {
        String::from(match self {
            Self::CurrentTemperature => "current_temperature",
            Self::Power => "power",
            Self::Mode => "mode",
            Self::FanMode => "fan_mode",
            Self::Temperature => "temperature",
            Self::VerticalSwingMode => "vertical_swing_mode",
            Self::SwingMode => "swing_mode",
        })
    }

    fn readable(&self) -> bool {
        match self {
            Self::CurrentTemperature => true,
            Self::Power => false,
            Self::Mode => true,
            Self::FanMode => true,
            Self::Temperature => true,
            Self::VerticalSwingMode => true,
            Self::SwingMode => true,
        }
    }

    fn writable(&self) -> bool {
        match self {
            Self::CurrentTemperature => false,
            Self::Power => true,
            Self::Mode => true,
            Self::FanMode => true,
            Self::Temperature => true,
            Self::VerticalSwingMode => true,
            Self::SwingMode => true,
        }
    }

    fn read_xform(&self, v: u32, raw_clip_state: &HashMap<u16, u32>) -> Option<String> {
        match self {
            Self::CurrentTemperature => Some((v / 2).to_string()),
            Self::Power => Some(String::from(if v == 0 { "OFF" } else { "ON" })),

            Self::Mode => {
                if raw_clip_state.get(&0x1f7) == Some(&0) {
                    Some(String::from("off"))
                } else {
                    match v {
                        0 => Some(String::from("cool")),
                        1 => Some(String::from("dry")),
                        2 => Some(String::from("fan_only")),
                        4 => Some(String::from("heat")),
                        6 => Some(String::from("auto")),
                        _ => None,
                    }
                }
            }

            Self::FanMode => match v {
                2 => Some(String::from("very low")),
                3 => Some(String::from("low")),
                4 => Some(String::from("medium")),
                5 => Some(String::from("high")),
                6 => Some(String::from("very high")),
                8 => Some(String::from("auto")),
                _ => None,
            },

            Self::Temperature => Some((v / 2).to_string()),

            Self::VerticalSwingMode => match v {
                0 => Some(String::from("off")),
                1..=6 => Some(v.to_string()),
                100 => Some(String::from("on")),
                _ => None,
            },

            Self::SwingMode => match v {
                0 => Some(String::from("off")),
                1..=5 => Some(v.to_string()),
                13 => Some(String::from("1-3")),
                35 => Some(String::from("3-5")),
                100 => Some(String::from("on")),
                _ => None,
            },
        }
    }

    fn read_callback(&self, _v: String) -> Option<u16> {
        match self {
            Self::Power => Some(0x1f9),
            _ => None,
        }
    }

    fn pre_write_xform_set_property(&self, v: String) -> Option<(String, String)> {
        match self {
            Self::Mode => {
                if v == "off" {
                    Some((String::from("power"), String::from("OFF")))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn write_xform(&self, v: String) -> Option<u32> {
        match self {
            Self::CurrentTemperature => None,
            Self::Power => Some(if v == "ON" { 1 } else { 0 }),
            Self::Mode => match v.as_str() {
                "cool" => Some(0),
                "dry" => Some(1),
                "fan_only" => Some(2),
                "heat" => Some(4),
                "auto" => Some(6),
                _ => None,
            },
            Self::FanMode => match v.as_str() {
                "very low" => Some(2),
                "low" => Some(3),
                "medium" => Some(4),
                "high" => Some(5),
                "very high" => Some(6),
                "auto" => Some(8),
                _ => None,
            },
            Self::Temperature => Some((v.parse::<f32>().unwrap() * 2.0).round() as u32),
            Self::VerticalSwingMode => match v.as_str() {
                "off" => Some(0),
                "1" => Some(1),
                "2" => Some(2),
                "3" => Some(3),
                "4" => Some(4),
                "5" => Some(5),
                "6" => Some(6),
                "on" => Some(100),
                _ => None,
            },
            Self::SwingMode => match v.as_str() {
                "off" => Some(0),
                "1" => Some(1),
                "2" => Some(2),
                "3" => Some(3),
                "4" => Some(4),
                "5" => Some(5),
                "1-3" => Some(13),
                "3-5" => Some(35),
                "on" => Some(100),
                _ => None,
            },
        }
    }

    fn write_callback(&self, _v: String) -> Option<()> {
        match self {
            _ => None,
        }
    }

    fn write_attach(&self, raw: u32) -> Option<Vec<u16>> {
        match self {
            Self::Power => Some(if raw == 0 { vec![] } else { vec![0x1f9, 0x1fa] }),
            Self::Mode => Some(vec![0x1fa, 0x1fe]),
            Self::FanMode => Some(vec![0x1f9, 0x1fe]),
            Self::Temperature => Some(vec![0x1f9, 0x1fa]),
            Self::VerticalSwingMode => Some(vec![0x1f9, 0x1fa]),
            Self::SwingMode => Some(vec![0x1f9, 0x1fa]),
            _ => None,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone)]
pub struct RAC_056905_WW;

impl HADevice for RAC_056905_WW {
    fn get_ha_class(&self) -> String {
        String::from("climate")
    }

    fn get_model(&self) -> String {
        String::from("RAC_056905_WW")
    }

    fn get_inner_config(
        &self,
        id: String,
        ponder_prefix: String,
    ) -> serde_json::Map<String, serde_json::Value> {
        json!({
            "name": "LG Air Conditioner",
            "temperature_unit": "C",
            "temp_step": 0.5,
            "precision": 0.5,
            "fan_modes": [ "auto", "very low", "low", "medium", "high", "very high" ],
            "swing_modes": [ "1", "2", "3", "4", "5", "1-3", "3-5", "on", "off" ],
            "vertical_swing_modes": [ "1", "2", "3", "4", "5", "6", "on", "off" ],
            "current_temperature_topic": format!("{}/{}/current_temperature", ponder_prefix, id),
            "power_command_topic": format!("{}/{}/power/set", ponder_prefix, id),
            "mode_state_topic": format!("{}/{}/mode", ponder_prefix, id),
            "mode_command_topic": format!("{}/{}/mode/set", ponder_prefix, id),
            "fan_mode_state_topic": format!("{}/{}/fan_mode", ponder_prefix, id),
            "fan_mode_command_topic": format!("{}/{}/fan_mode/set", ponder_prefix, id),
            "temperature_state_topic": format!("{}/{}/temperature", ponder_prefix, id),
            "temperature_command_topic": format!("{}/{}/temperature/set", ponder_prefix, id),
            "swing_mode_state_topic": format!("{}/{}/swing_mode", ponder_prefix, id),
            "swing_mode_command_topic": format!("{}/{}/swing_mode/set", ponder_prefix, id),
        })
        .as_object()
        .unwrap()
        .clone()
    }

    fn get_field_by_id(&self, t: u16) -> Option<Box<dyn Field>> {
        RAC_056905_WW_Fields::from_id(t).map(|f| Box::new(f) as Box<dyn Field>)
    }

    fn get_field_by_ha(&self, prop: String) -> Option<Box<dyn Field>> {
        RAC_056905_WW_Fields::from_name(&prop).map(|f| Box::new(f) as Box<dyn Field>)
    }
}
