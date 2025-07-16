use rmqtt::context::ServerContext;
use rumqttc::AsyncClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{device::DeviceWrapper, tlv::parse_tlv};

pub struct DeviceManager {
    pub devices: HashMap<String, DeviceWrapper>,
    pub deploy_msg_list: HashMap<String, String>,

    pub scx: ServerContext,
    pub ha_mqtt_client: AsyncClient,

    pub discovery_prefix: String,
    pub ponder_prefix: String,
}

#[derive(Serialize, Deserialize)]
pub struct Payload {
    pub cmd: String,
    pub did: String,
    pub kind: String,
    pub data: serde_json::Value,
}

impl DeviceManager {
    pub fn new(
        scx: ServerContext,
        ha_mqtt_client: AsyncClient,
        discovery_prefix: String,
        ponder_prefix: String,
    ) -> Self {
        Self {
            devices: HashMap::default(),
            deploy_msg_list: HashMap::default(),

            scx,
            ha_mqtt_client,

            discovery_prefix,
            ponder_prefix,
        }
    }

    pub async fn on_publish(&mut self, topic: String, payload_serialized: String) {
        // eprintln!("\ntopic: {}\npayload: {}", topic, payload_serialized);

        if topic.starts_with("clip/") {
            let payload: Payload =
                serde_json::from_str(&payload_serialized.trim_end_matches("\0")).unwrap();

            if topic == format!("clip/message/devices/{}", payload.did) {
                if payload.cmd == "completeProvisioning_ack" {
                    self.complete_provisioning(payload.did.clone(), payload.kind.clone())
                        .await;
                }

                if payload.cmd == "device_packet" {
                    if let Some(device) = self.devices.get_mut(&payload.did) {
                        let buf = hex::decode(payload.data.as_str().unwrap()).unwrap();

                        // eprintln!("buf: {:X?} | buf.len() - 13: {}", buf, buf.len() - 13);

                        if buf[2] == 0x04
                            && buf[3] == 0x00
                            && buf[4] == 0x00
                            && buf[5] == 0x00
                            && (buf[6] == 0x87 || buf[6] == 0xA7) // RAC sends 0x87 but CST sends 0xA7
                            && buf[7] == 0x02
                            && buf[8] == 0x04
                            && buf[10] == (buf.len() - 13) as u8
                        {
                            let tlv = parse_tlv(&buf[11..buf.len() - 2]);

                            // eprintln!("\nTLV: {:?}", tlv);

                            device.process_tlv(self.ponder_prefix.clone(), tlv).await;
                        }
                    }
                }
            }

            if topic == format!("clip/provisioning/devices/{}", payload.did) {
                if payload.cmd == "preDeploy" || payload.cmd == "deploy" {
                    self.deploy_msg_list
                        .insert(payload.did.clone(), payload_serialized);

                    let from = rmqtt::types::From::from_custom(rmqtt::types::Id::new(
                        self.scx.node.id(),
                        0,
                        None,
                        None,
                        rmqtt::types::ClientId::new(),
                        None,
                    ));

                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64;

                    let publish = rmqtt::codec::types::Publish {
                        topic: format!("lime/devices/{}", payload.did).into(),
                        retain: false,
                        qos: rmqtt::codec::types::QoS::AtMostOnce,
                        dup: false,
                        payload: deploy_response(payload, timestamp).into(),
                        packet_id: None,
                        properties: Some(Default::default()),
                        delay_interval: None,
                        create_time: Some(timestamp),
                    };

                    let message = Box::new(publish);

                    let message = self
                        .scx
                        .extends
                        .hook_mgr()
                        .message_publish(None, from.clone(), &message)
                        .await
                        .unwrap_or(message);

                    if let Err(e) = rmqtt::session::SessionState::forwards(
                        &self.scx, from, message, false, None,
                    )
                    .await
                    {
                        eprintln!("Error forwarding message: {e:?}");
                    }
                }
            }
        }
    }

    async fn complete_provisioning(&mut self, device_id: String, kind: String) {
        if self.deploy_msg_list.get(&device_id).is_none() {
            eprintln!("completeProvisioning_ack received without deploy/preDeploy");
            return;
        }

        if self.devices.get(&device_id).is_some() {
            eprintln!("completeProvisioning_ack received twice?");
            return;
        }

        let dev = DeviceWrapper::new(
            self.scx.clone(),
            self.ha_mqtt_client.clone(),
            self.discovery_prefix.clone(),
            self.ponder_prefix.clone(),
            kind,
            device_id.clone(),
            format!("lime/devices/{}", device_id),
        )
        .await;

        self.devices.insert(device_id.clone(), dev);

        println!("Device {} started", device_id);
    }

    pub async fn on_discovery(&self) {
        for dev in self.devices.values() {
            dev.publish_config(self.discovery_prefix.clone(), self.ponder_prefix.clone())
                .await
        }
    }

    pub async fn on_set_property(&mut self, id: String, prop: String, value: String) {
        if let Some(dev) = self.devices.get_mut(&id) {
            dev.set_property(prop, value).await;
        }
    }
}

fn deploy_response(payload: Payload, timestamp: i64) -> String {
    let json = serde_json::json!({
        "did": payload.did,
        "mid": timestamp,
        "cmd": "completeProvisioning",
        "type":0,
        "data": {
            "result":0,
            "host": "message",
            "appInfo": {
                "host":"message",
                "publication":{
                    "message": format!("clip/message/devices/{}", payload.did),
                    "provisioning": format!("clip/provisioning/devices/{}", payload.did)
                }
            },
            "provisioningType": payload.cmd,
            "deployInterval":600

        }
    });

    json.to_string()
}
