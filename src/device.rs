use rmqtt::context::ServerContext;
use rumqttc::AsyncClient;
use serde_json::json;
use std::collections::HashMap;

use crate::{
    crc16::crc16,
    tlv::{build_tlv, Tlv},
};

#[allow(non_camel_case_types)]
#[derive(Clone)]
pub enum DeviceTypes {
    RAC_056905_WW,
    CST_570004_WW,
}

impl DeviceTypes {
    fn get_ha_class(&self) -> String {
        match self {
            Self::RAC_056905_WW => crate::devices::RAC_056905_WW::RAC_056905_WW.get_ha_class(),
            Self::CST_570004_WW => crate::devices::CST_570004_WW::CST_570004_WW.get_ha_class(),
        }
    }

    fn get_model(&self) -> String {
        match self {
            Self::RAC_056905_WW => crate::devices::RAC_056905_WW::RAC_056905_WW.get_model(),
            Self::CST_570004_WW => crate::devices::CST_570004_WW::CST_570004_WW.get_model(),
        }
    }

    fn get_inner_config(
        &self,
        id: String,
        ponder_prefix: String,
    ) -> serde_json::Map<String, serde_json::Value> {
        match self {
            Self::RAC_056905_WW => {
                crate::devices::RAC_056905_WW::RAC_056905_WW.get_inner_config(id, ponder_prefix)
            }
            Self::CST_570004_WW => {
                crate::devices::CST_570004_WW::CST_570004_WW.get_inner_config(id, ponder_prefix)
            }
        }
    }

    fn get_field_by_id(&self, t: u16) -> Option<Box<dyn Field>> {
        match self {
            Self::RAC_056905_WW => crate::devices::RAC_056905_WW::RAC_056905_WW.get_field_by_id(t),
            Self::CST_570004_WW => crate::devices::CST_570004_WW::CST_570004_WW.get_field_by_id(t),
        }
    }

    fn get_field_by_ha(&self, prop: String) -> Option<Box<dyn Field>> {
        match self {
            Self::RAC_056905_WW => {
                crate::devices::RAC_056905_WW::RAC_056905_WW.get_field_by_ha(prop)
            }
            Self::CST_570004_WW => {
                crate::devices::CST_570004_WW::CST_570004_WW.get_field_by_ha(prop)
            }
        }
    }
}

#[derive(Clone)]
pub struct DeviceWrapper {
    scx: ServerContext,
    id: String,
    topic: String,
    raw_clip_state: HashMap<u16, u32>,
    device: DeviceTypes,
    ha_mqtt_client: AsyncClient,
}

impl DeviceWrapper {
    async fn init(&self, discovery_prefix: String, ponder_prefix: String) {
        self.publish_config(discovery_prefix, ponder_prefix).await;
        self.query().await;
    }

    pub async fn new(
        scx: ServerContext,
        ha_mqtt_client: AsyncClient,
        discovery_prefix: String,
        ponder_prefix: String,
        kind: String,
        id: String,
        topic: String,
    ) -> Self {
        let device = match kind.as_str() {
            "RAC_056905_WW" => DeviceTypes::RAC_056905_WW,
            "CST_570004_WW" => DeviceTypes::CST_570004_WW,
            _ => panic!("unknown device"),
        };

        let s = Self {
            scx,
            id,
            topic,
            raw_clip_state: HashMap::new(),
            device,
            ha_mqtt_client,
        };

        s.init(discovery_prefix, ponder_prefix).await;

        return s;
    }

    async fn pre_set_property(&mut self, prop: String, value: String) {
        let mut raw_clip_state = None;

        if let Some(def) = self.device.get_field_by_ha(prop) {
            if def.writable() {
                let new_value = def.write_xform(value.clone());

                if let Some(new_v) = new_value {
                    if let None = def.write_callback(value) {
                        raw_clip_state = Some((def.id(), new_v));

                        let mut attach = Vec::new();

                        if let Some(array) = def.write_attach(new_v) {
                            attach = array;
                        }

                        let write_fields = [&[def.id()], attach.as_slice()].concat();

                        let tlv: Vec<Tlv> = write_fields
                            .into_iter()
                            .map(|id| Tlv {
                                t: id,
                                v: if id == def.id() {
                                    new_v
                                } else {
                                    self.get_raw_clip_state(id).unwrap()
                                },
                            })
                            .collect();

                        self.send([1, 1, 2, 1, 1], tlv).await;
                    }
                }
            }
        }

        if let Some((id, value)) = raw_clip_state {
            self.set_raw_clip_state(id, value);
        }
    }

    pub async fn set_property(&mut self, prop: String, value: String) {
        let mut raw_clip_state = None;

        let clone = self.clone();
        let maybe_field = clone.device.get_field_by_ha(prop);

        if let Some(def) = maybe_field {
            if def.writable() {
                if let Some((p, v)) = def.pre_write_xform_set_property(value.clone()) {
                    self.pre_set_property(p, v).await;
                }

                let new_value = def.write_xform(value.clone());

                if let Some(new_v) = new_value {
                    if let None = def.write_callback(value) {
                        raw_clip_state = Some((def.id(), new_v));

                        let mut attach = Vec::new();

                        if let Some(array) = def.write_attach(new_v) {
                            attach = array;
                        }

                        let write_fields = [&[def.id()], attach.as_slice()].concat();

                        let tlv: Vec<Tlv> = write_fields
                            .into_iter()
                            .map(|id| Tlv {
                                t: id,
                                v: if id == def.id() {
                                    new_v
                                } else {
                                    // eprintln!("get raw clip state for id: {:X}", id);
                                    self.get_raw_clip_state(id).unwrap()
                                },
                            })
                            .collect();

                        self.send([1, 1, 2, 1, 1], tlv).await;
                    }
                }
            }
        }

        if let Some((id, value)) = raw_clip_state {
            self.set_raw_clip_state(id, value);
        }
    }

    async fn send(&self, header: [u8; 5], tlv: Vec<Tlv>) {
        let [b0, b1, b2, b3, b4] = header;

        let tlv_buf = build_tlv(&tlv);

        let mut buf = [
            &[
                0x04,
                0x00,
                0x00,
                0x00,
                0x65,
                b2,
                b3,
                b4,
                tlv_buf.len() as u8,
            ],
            tlv_buf.as_slice(),
        ]
        .concat();

        let result = crc16(&buf);

        buf = [
            &[b0, b1],
            buf.as_slice(),
            &[((result >> 8) as u8), (result as u8 & 0xff)],
        ]
        .concat();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let message_str = json!({
            "did": self.get_id(),
            "mid": timestamp,
            "cmd": "packet",
            "type": 1,
            "data": hex::encode(&buf)
        })
        .to_string();

        let from = rmqtt::types::From::from_custom(rmqtt::types::Id::new(
            self.scx.node.id(),
            0,
            None,
            None,
            rmqtt::types::ClientId::new(),
            None,
        ));

        let message = Box::new(rmqtt::codec::types::Publish {
            topic: self.get_topic().into(),
            retain: false,
            qos: rmqtt::codec::types::QoS::AtMostOnce,
            dup: false,
            payload: message_str.into(),
            packet_id: None,
            properties: Some(Default::default()),
            delay_interval: None,
            create_time: Some(timestamp),
        });

        let message = self
            .scx
            .extends
            .hook_mgr()
            .message_publish(None, from.clone(), &message)
            .await
            .unwrap_or(message);

        if let Err(e) =
            rmqtt::session::SessionState::forwards(&self.scx, from, message, false, None).await
        {
            eprintln!("Error forwarding message: {e:?}");
        }
    }

    async fn query(&self) {
        self.send([1, 1, 2, 2, 1], vec![Tlv { t: 0x1f5, v: 2 }])
            .await
    }

    async fn ha_publish_config(&self, discovery_prefix: String, ponder_prefix: String) {
        let id = self.get_id();

        let discovery_topic_config = format!(
            "{}/{}/{}/{}/config",
            discovery_prefix,
            self.device.get_ha_class(),
            ponder_prefix,
            id
        );

        let config = self.get_config(ponder_prefix);

        self.publish_to_ha(discovery_topic_config, config, false)
            .await;
    }

    async fn ha_publish_property(
        &self,
        ponder_prefix: String,
        id: String,
        property: String,
        value: String,
        retain: bool,
    ) {
        // eprintln!(
        //     "ha_publish_property id: {}, property: {}, value: {}, retain: {}",
        //     id, property, value, retain
        // );

        let device_topic_property = format!("{}/{}/{}", ponder_prefix, id, property);

        self.publish_to_ha(device_topic_property, value, retain)
            .await;
    }

    async fn publish_to_ha(&self, topic: String, payload: String, retain: bool) {
        self.ha_mqtt_client
            .publish(topic, rumqttc::QoS::AtMostOnce, retain, payload)
            .await
            .unwrap();
    }

    pub async fn process_tlv(&mut self, ponder_prefix: String, tlv: Vec<Tlv>) {
        for Tlv { t, v } in tlv {
            self.process_key_value(ponder_prefix.clone(), t, v).await;
        }
    }

    async fn process_key_value(&mut self, ponder_prefix: String, mut t: u16, v: u32) {
        loop {
            self.set_raw_clip_state(t, v);

            // eprintln!(
            //     "{} set raw clip state: t: {:X}, v: {}",
            //     self.device.get_model(),
            //     t,
            //     v
            // );

            let clone = self.clone();
            let maybe_field = clone.device.get_field_by_id(t);

            if let Some(def) = maybe_field {
                let new_v = def
                    .read_xform(v, &self.raw_clip_state())
                    .unwrap_or(v.to_string());

                if let Some(new_t) = def.read_callback(new_v.clone()) {
                    t = new_t;

                    continue;
                } else {
                    if def.readable() {
                        self.ha_publish_property(
                            ponder_prefix,
                            self.get_id(),
                            def.name(),
                            new_v,
                            true,
                        )
                        .await
                    }
                    break;
                }
            } else {
                break;
            }
        }
    }

    pub async fn publish_config(&self, discovery_prefix: String, ponder_prefix: String) {
        self.ha_publish_config(discovery_prefix, ponder_prefix.clone())
            .await;

        self.ha_publish_property(
            ponder_prefix,
            self.get_id(),
            String::from("availability"),
            String::from("online"),
            false,
        )
        .await;
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }

    fn get_topic(&self) -> String {
        self.topic.clone()
    }

    fn raw_clip_state(&self) -> HashMap<u16, u32> {
        self.raw_clip_state.clone()
    }

    fn get_raw_clip_state(&self, t: u16) -> Option<u32> {
        self.raw_clip_state.get(&t).copied()
    }

    fn set_raw_clip_state(&mut self, t: u16, v: u32) {
        self.raw_clip_state.insert(t, v);
    }

    fn get_config(&self, ponder_prefix: String) -> String {
        let id = self.get_id();

        let mut inner_config = self
            .device
            .get_inner_config(id.clone(), ponder_prefix.clone());

        let mut value = json!({
            "availability": [ { "topic": format!("{}/{}/availability", ponder_prefix, id) }, { "topic": format!("{}/availability", ponder_prefix) } ],
            "optimistic": false,
            "object_id": id,
            "unique_id": id,
            "device": {
                "identifiers": id,
                "manufacturer": "LG",
                "model": self.device.get_model(),
                "sw_version": "885612", // TODO: Figure out if this is really needed and if so pass it through from device manager.
            },
        });

        value.as_object_mut().unwrap().append(&mut inner_config);

        value.to_string()
    }
}

pub trait Field: Send {
    fn id(&self) -> u16;

    fn name(&self) -> String;

    fn readable(&self) -> bool;

    fn writable(&self) -> bool;

    fn read_xform(&self, v: u32, raw_clip_state: &HashMap<u16, u32>) -> Option<String>;
    fn read_callback(&self, v: String) -> Option<u16>;

    fn pre_write_xform_set_property(&self, v: String) -> Option<(String, String)>;
    fn write_xform(&self, v: String) -> Option<u32>;
    fn write_callback(&self, v: String) -> Option<()>;

    fn write_attach(&self, raw: u32) -> Option<Vec<u16>>;
}

pub trait HADevice: Clone {
    fn get_ha_class(&self) -> String;

    fn get_inner_config(
        &self,
        id: String,
        ponder_prefix: String,
    ) -> serde_json::Map<String, serde_json::Value>;

    fn get_model(&self) -> String;

    fn get_field_by_id(&self, t: u16) -> Option<Box<dyn Field>>;

    fn get_field_by_ha(&self, prop: String) -> Option<Box<dyn Field>>;
}
