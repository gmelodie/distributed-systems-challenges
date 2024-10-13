use std::{
    collections::HashSet,
    io::{self, Read},
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

struct Node {
    id: String,
    node_ids: HashSet<String>,
}

impl Node {
    fn init(&mut self, id: String, node_ids: HashSet<String>) {
        self.id = id;
        self.node_ids = node_ids;
    }
    fn process(&mut self, msg: Message) -> Result<Message> {
        if msg.body.is_none() {
            if msg.ty.unwrap().as_str() == "init" {
                self.init(
                    msg.node_id.unwrap(),
                    msg.node_ids.unwrap().into_iter().collect(), // vec to hashset
                );

                return Ok(Message {
                    ty: Some("init_ok".to_string()),
                    in_reply_to: msg.id,
                    src: None,
                    dst: None,
                    id: None,
                    body: None,
                    node_id: None,
                    node_ids: None,
                });
            }
        }
        let body = msg.body.unwrap();
        match body.ty.as_str() {
            "echo" => Ok(Message {
                src: msg.dst,
                dst: msg.src,
                body: Some(Body {
                    ty: "echo_ok".to_string(),
                    id: body.id,
                    in_reply_to: Some(body.id),
                    echo: body.echo,
                }),
                ty: None,
                in_reply_to: None,
                id: None,
                node_id: None,
                node_ids: None,
            }),
            _ => panic!("Unrecognized msg type: {}", body.ty),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    src: Option<String>,
    #[serde(rename = "dest")]
    dst: Option<String>,
    #[serde(rename = "type")]
    ty: Option<String>,
    in_reply_to: Option<usize>,
    body: Option<Body>,
    #[serde(rename = "msg_id")]
    id: Option<usize>,
    node_id: Option<String>,
    node_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Body {
    #[serde(rename = "type")]
    ty: String,
    #[serde(rename = "msg_id")]
    id: usize,
    in_reply_to: Option<usize>,
    echo: Option<String>,
}

fn main() -> Result<()> {
    let mut node = Node {
        id: String::new(),
        node_ids: HashSet::new(),
    };
    loop {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let msg: Message = serde_json::from_str(&input)?;
        println!("{:#?}", msg);
        let resp = node.process(msg)?;
        println!("{}", serde_json::to_string(&resp)?);
    }

    Ok(())
}
