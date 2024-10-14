use std::{
    collections::{HashMap, HashSet},
    io,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

struct Node {
    id: String,
    node_ids: HashSet<String>,
    messages: HashSet<usize>,
    g_counter: AtomicUsize,
}

impl Node {
    fn from_init(msg: Message) -> Result<(Message, Self)> {
        match msg.body.payload {
            Payload::Init { node_id, node_ids } => Ok((
                Message {
                    src: msg.dst,
                    dst: msg.src,
                    body: Body {
                        id: None,
                        in_reply_to: msg.body.id,
                        payload: Payload::InitOk {},
                    },
                },
                Self {
                    id: node_id,
                    node_ids: node_ids.into_iter().collect(),
                    messages: HashSet::<usize>::new(),
                    g_counter: AtomicUsize::new(0),
                },
            )),
            _ => Err(anyhow!("Message is not init type")),
        }
    }

    fn generate_uuid(&mut self) -> String {
        Uuid::new_v4().hyphenated().to_string()
    }

    fn process(&mut self, msg: Message) -> Result<Message> {
        // if !self.node_ids.contains(&msg.src) || !self.node_ids.contains(&msg.dst) {
        //     return Err(anyhow!("Src or Dst not in node_ids"));
        // }
        if msg.dst != self.id {
            return Ok(Message {
                src: self.id.clone(),
                dst: msg.src,
                body: Body {
                    id: None,
                    in_reply_to: msg.body.id,
                    payload: Payload::Error {
                        code: 1001, // 1000 and above are for our own uses
                        text: "Destination does not match this node_id".to_string(),
                    },
                },
            });
        }
        match msg.body.payload {
            Payload::Init {
                node_id: _,
                node_ids: _,
            } => Ok(Message {
                src: self.id.clone(),
                dst: msg.src,
                body: Body {
                    id: None,
                    in_reply_to: msg.body.id,
                    payload: Payload::Error {
                        code: 1002,
                        text: "Node already initialized".to_string(),
                    },
                },
            }),
            Payload::Echo { echo } => Ok(Message {
                src: self.id.clone(),
                dst: msg.src,
                body: Body {
                    id: msg.body.id,
                    in_reply_to: msg.body.id,
                    payload: Payload::EchoOk { echo },
                },
            }),
            Payload::Generate {} => {
                let uuid = self.generate_uuid();
                Ok(Message {
                    src: self.id.clone(),
                    dst: msg.src,
                    body: Body {
                        id: msg.body.id,
                        in_reply_to: msg.body.id,
                        payload: Payload::GenerateOk { id: uuid },
                    },
                })
            }
            Payload::Broadcast { message } => {
                self.messages.insert(message);
                Ok(Message {
                    src: self.id.clone(),
                    dst: msg.src,
                    body: Body {
                        id: msg.body.id,
                        in_reply_to: msg.body.id,
                        payload: Payload::BroadcastOk {},
                    },
                })
            }
            // // This read is for topology
            // Payload::Read {} => Ok(Message {
            //     src: self.id.clone(),
            //     dst: msg.src,
            //     body: Body {
            //         id: msg.body.id,
            //         in_reply_to: msg.body.id,
            //         payload: Payload::ReadOk {
            //             messages: self.messages.clone().into_iter().collect(),
            //         },
            //     },
            // }),
            // This read is for global counter
            Payload::Read {} => {
                let value = self.g_counter.load(Ordering::SeqCst);
                Ok(Message {
                    src: self.id.clone(),
                    dst: msg.src,
                    body: Body {
                        id: msg.body.id,
                        in_reply_to: msg.body.id,
                        payload: Payload::ReadOk { value },
                    },
                })
            }
            Payload::Topology { topology: _ } => Ok(Message {
                src: self.id.clone(),
                dst: msg.src,
                body: Body {
                    id: msg.body.id,
                    in_reply_to: msg.body.id,
                    payload: Payload::TopologyOk {},
                },
            }),
            Payload::Add { delta } => {
                let _value = self.g_counter.fetch_add(delta, Ordering::Relaxed);
                Ok(Message {
                    src: self.id.clone(),
                    dst: msg.src,
                    body: Body {
                        id: msg.body.id,
                        in_reply_to: msg.body.id,
                        payload: Payload::AddOk {},
                    },
                })
            }
            _ => panic!("Unrecognized msg type"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    src: String,
    #[serde(rename = "dest")]
    dst: String,
    body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Body {
    #[serde(rename = "msg_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    in_reply_to: Option<usize>,
    #[serde(flatten)]
    payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Echo {
        echo: String,
    },
    EchoOk {
        echo: String,
    },
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk {},

    Generate {},
    GenerateOk {
        id: String,
    },

    Broadcast {
        message: usize,
    },
    BroadcastOk {},

    // Topology Read
    // Read {},
    // ReadOk {
    //     messages: Vec<usize>,
    // },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk {},

    // Add Read
    Read {},
    ReadOk {
        value: usize,
    },

    Add {
        delta: usize,
    },
    AddOk {},

    Error {
        code: usize,
        text: String,
    },
}

fn main() -> Result<()> {
    let mut node = Node {
        id: "n0".to_string(),
        node_ids: HashSet::new(),
        messages: HashSet::new(),
        g_counter: AtomicUsize::new(0),
    };

    for (i, line) in io::stdin().lines().enumerate() {
        let msg: Message = serde_json::from_str(&line?)?;

        let resp = if i == 0 {
            let (resp, new_node) = Node::from_init(msg)?;
            node = new_node;
            resp
        } else {
            node.process(msg)?
        };

        println!("{}", serde_json::to_string(&resp)?);
    }

    Ok(())
}
