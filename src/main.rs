use async_trait::async_trait;
use device_manager::DeviceManager;
use rmqtt::{
    context::ServerContext,
    hook::{Handler, HookResult, Parameter, Priority, Register, ReturnType, Type},
    macros::Plugin,
    net::Builder,
    plugin::Plugin,
    server::MqttServer,
    Result,
};
use rumqttc::{AsyncClient, MqttOptions};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};

mod crc16;
mod device;
mod device_manager;
mod devices;
mod tlv;

struct PublishHandler {
    tx: Sender<(String, String)>,
}

impl PublishHandler {
    fn new(tx: &Sender<(String, String)>) -> Self {
        Self { tx: tx.clone() }
    }
}

#[async_trait]
impl Handler for PublishHandler {
    async fn hook(&self, param: &Parameter, acc: Option<HookResult>) -> ReturnType {
        if let Parameter::MessagePublish(_, _, publish) = param {
            let topic = &publish.topic;
            let payload = std::str::from_utf8(&publish.payload).unwrap_or("<binary>");

            self.tx
                .send((topic.to_string(), payload.to_string()))
                .await
                .unwrap();
        }

        if let Parameter::ClientConnect(_) = param {
            return (
                true,
                Some(HookResult::ConnectAckReason(
                    rmqtt::types::ConnectAckReason::V3(
                        rmqtt::codec::v3::ConnectAckReason::ConnectionAccepted,
                    ),
                )),
            );
        }

        (true, acc)
    }
}

#[inline]
pub async fn register_named(
    scx: &rmqtt::context::ServerContext,
    tx: Sender<(String, String)>,
    name: &'static str,
    default_startup: bool,
    immutable: bool,
) -> rmqtt::Result<()> {
    let scx1 = scx.clone();
    let tx1 = tx.clone();
    scx.plugins
        .register(
            name,
            default_startup,
            immutable,
            move || -> rmqtt::plugin::DynPluginResult {
                let scx1 = scx1.clone();
                let tx1 = tx1.clone();
                Box::pin(async move {
                    PublishHookPlugin::new(scx1.clone(), tx1.clone(), name)
                        .await
                        .map(|p| -> rmqtt::plugin::DynPlugin { Box::new(p) })
                })
            },
        )
        .await?;

    Ok(())
}

#[inline]
pub async fn register(
    scx: &rmqtt::context::ServerContext,
    tx: Sender<(String, String)>,
    default_startup: bool,
    immutable: bool,
) -> rmqtt::Result<()> {
    register_named(scx, tx, "PublishHookPlugin", default_startup, immutable).await
}

#[derive(Plugin)]
struct PublishHookPlugin {
    tx: Sender<(String, String)>,
    register: Box<dyn Register>,
}

impl PublishHookPlugin {
    #[inline]
    async fn new<S: Into<String>>(
        scx: ServerContext,
        tx: Sender<(String, String)>,
        _name: S,
    ) -> Result<Self> {
        let register = scx.extends.hook_mgr().register();
        Ok(Self { tx, register })
    }
}

#[async_trait]
impl Plugin for PublishHookPlugin {
    #[inline]
    async fn init(&mut self) -> Result<()> {
        self.register
            .add_priority(
                Type::MessagePublish,
                Priority::MAX,
                Box::new(PublishHandler::new(&self.tx)),
            )
            .await;

        Ok(())
    }

    #[inline]
    async fn load_config(&mut self) -> Result<()> {
        Ok(())
    }

    #[inline]
    async fn start(&mut self) -> Result<()> {
        self.register.start().await;
        Ok(())
    }

    #[inline]
    async fn stop(&mut self) -> Result<bool> {
        Ok(false)
    }
}

#[derive(Debug, Deserialize)]
pub struct HAConf {
    address: String,
    port: u16,
    username: String,
    password: String,
    ponder_prefix: String,
    discovery_prefix: String,
}

#[derive(Debug, Deserialize)]
pub struct Conf {
    home_assistant: HAConf,
    ca_cert_file: String,
    ca_key_file: String,
    #[allow(dead_code)]
    https_port: u16,
    mqtts_port: u16,
    mqtt_port: u16,
    #[allow(dead_code)]
    hostname: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let config: Conf = config::Config::builder()
        .add_source(config::File::with_name("./config.toml"))
        .build()?
        .try_deserialize()?;

    let (tx, mut rx) = mpsc::channel::<(String, String)>(100);

    // TODO: Implement provisioning.
    // tokio::spawn(async move {
    //     let mut app = tide::new();

    //     app.listen(
    //         tide_rustls::TlsListener::build()
    //             .addrs("ponder.lan:4433")
    //             .cert(String::from("./ca.cert"))
    //             .key(String::from("./ca.key")),
    //     )
    //     .await
    //     .unwrap();
    // });

    let scx = ServerContext::new().build().await;

    register(&scx, tx, true, false).await.unwrap();

    MqttServer::new(scx.clone())
        .listener(
            Builder::new()
                .name("external/tcp")
                .laddr(([0, 0, 0, 0], config.mqtts_port).into())
                // TODO: Generate certs if they don't exist.
                .tls_cert(Some(config.ca_cert_file))
                .tls_key(Some(config.ca_key_file))
                .bind()?
                .tls()?,
        )
        .listener(
            Builder::new()
                .name("/tcp")
                .laddr(([0, 0, 0, 0], config.mqtt_port).into())
                .bind()?
                .tcp()?,
        )
        .build()
        .start();

    let mut mqttoptions = MqttOptions::new(
        "ponder",
        config.home_assistant.address,
        config.home_assistant.port,
    );
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_credentials(
        config.home_assistant.username,
        config.home_assistant.password,
    );
    mqttoptions.set_last_will(rumqttc::LastWill {
        topic: format!("{}/availability", config.home_assistant.ponder_prefix),
        message: "offline".into(),
        qos: rumqttc::QoS::AtMostOnce,
        retain: false,
    });

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    client
        .subscribe(
            format!("{}/status", config.home_assistant.discovery_prefix),
            rumqttc::QoS::AtMostOnce,
        )
        .await
        .unwrap();
    client
        .subscribe(
            format!("{}/+/+/set", config.home_assistant.ponder_prefix),
            rumqttc::QoS::AtMostOnce,
        )
        .await
        .unwrap();

    let device_manager = DeviceManager::new(
        scx,
        client.clone(),
        config.home_assistant.discovery_prefix.clone(),
        config.home_assistant.ponder_prefix.clone(),
    );

    let device_manager_1 = Arc::new(Mutex::new(device_manager));
    let device_manager_2 = device_manager_1.clone();

    tokio::spawn(async move {
        while let Ok(notification) = eventloop.poll().await {
            if let rumqttc::Event::Incoming(rumqttc::Incoming::Publish(rumqttc::Publish {
                topic,
                payload,
                ..
            })) = notification
            {
                if topic
                    == String::from(format!("{}/status", config.home_assistant.discovery_prefix))
                    && payload == String::from("online")
                {
                    println!("HA online, starting discovery process");

                    device_manager_1.clone().lock().await.on_discovery().await;
                }

                if topic.starts_with(format!("{}/", config.home_assistant.ponder_prefix).as_str()) {
                    let path_elements: Vec<&str> = topic
                        [(config.home_assistant.ponder_prefix.len() + 1)..]
                        .split("/")
                        .collect();

                    if path_elements.len() == 3 && path_elements[2] == "set" {
                        let id = path_elements[0];
                        let prop = path_elements[1];

                        device_manager_1
                            .clone()
                            .lock()
                            .await
                            .on_set_property(
                                id.to_string(),
                                prop.to_string(),
                                String::from_utf8(payload.to_vec()).unwrap(),
                            )
                            .await;
                    }
                }
            }
        }
    });

    while let Some((topic, payload)) = rx.recv().await {
        device_manager_2
            .lock()
            .await
            .on_publish(topic, payload)
            .await;
    }

    Ok(())
}
