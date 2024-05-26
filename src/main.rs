use anyhow::Context;
use serde::{self, Deserialize, Serialize};
use std::io::{stdin, stdout, StdoutLock, Write};

// Thanks Jon Gjengset for serde and I/O stuff lol
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
    id: Option<u64>,
    #[serde(rename = "in_reply_to")]
    reply_to: Option<u64>,
    // Flattens the keys in the Payload into the Body.
    #[serde(flatten)]
    payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// "Tags" the enum struct with the provided string, like { "type": "echo", .. }
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
    InitOk,
}

// TODO: spin up all nodes with an Init message somewhere
struct EchoNode {
    node_id: Option<String>,
    msg_id: u64,
}

impl EchoNode {
    pub fn handle<'a>(&mut self, input: Message, output: &mut StdoutLock) -> anyhow::Result<()> {
        self.msg_id = if let Some(incoming_id) = input.body.id {
            std::cmp::max(incoming_id, self.msg_id)
        } else {
            self.msg_id
        } + 1;

        let reply = match input.body.payload {
            Payload::Echo { echo } => {
                let body = Body {
                    id: Some(self.msg_id),
                    reply_to: input.body.id,
                    payload: Payload::EchoOk { echo },
                };

                Some(Message {
                    src: input.dst,
                    dst: input.src,
                    body,
                })
            }
            Payload::Init { node_id, .. } => {
                self.node_id = Some(node_id);

                Some(Message {
                    src: self.node_id.as_ref().unwrap().to_string(),
                    dst: input.src,
                    body: Body {
                        id: Some(self.msg_id),
                        reply_to: input.body.id,
                        payload: Payload::InitOk,
                    },
                })
            }
            _ => None,
        };

        serde_json::to_writer(&mut *output, &reply).context("serialize response")?;
        output.write_all(b"\n").context("serialize newline")?;

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let stdin = stdin();
    // Receive Maelstrom messages over STDIN
    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message>();

    // Send Malestrom messages over STDOUT. Can't construct serializer because they're
    // newline-delimited.
    let mut stdout = stdout().lock();

    // TODO: dynamically initialize nodes in response to an Init message
    let mut node = EchoNode {
        node_id: None,
        msg_id: 0,
    };

    for input in inputs {
        let input = input.context("Malestrom input from STDIN could not be deserialized")?;

        node.handle(input, &mut stdout)
            .context("node handle function failed")?;
    }

    Ok(())
}
