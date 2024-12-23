use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

use wg_2024::{
    packet::Packet,
    network::NodeId,
};

/// From controller to Server
#[derive(Debug, Clone)]
pub enum ServerCommand {
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>)
}

///Server-Controller
pub enum ServerEvent {
}

/// From controller to Client
pub enum ClientCommand {
    //Controller functions
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>),

    //Client behaviour
    AskTypeTo(NodeId),
    StartFlooding,
}

pub enum ClientEvent {

}

//Queries (Client -> Server)
#[derive(Deserialize, Serialize, Debug)]
pub enum Query{
    //Common-shared
    AskType,

    //To Communication Server
    AddClient(String, NodeId),
    AskListClients,
    SendMessageTo(String, Message),

    //To Content Server
    //(Text)
    AskListFiles,
    AskFile(u8),
    //(Media)
    AskMedia(String), // String is the reference found in the files
}

//Server -> Client
#[derive(Deserialize, Serialize, Debug)]
pub enum Response {
    //Common-shared
    ServerType(ServerType),

    //From Communication Server
    MessageFrom(String, Message),
    ListUsers(Vec<String>),

    //From Content Server
    //(Text)
    ListFiles(Vec<String>),
    File(String),
    //(Media)
    Media(String),

    //General Error
    Err(String)
}

///Material
#[derive(Deserialize, Serialize, Copy, Clone, Debug, PartialEq)]
pub enum ServerType {
    Communication,
    Text,
    Media,
}

#[derive(Deserialize, Serialize, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Message {
    pub text: String,
}
