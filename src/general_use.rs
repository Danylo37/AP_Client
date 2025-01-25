use std::fmt::{Display, Formatter};
use crossbeam_channel::Sender;
use serde::{Deserialize, Serialize};

use wg_2024::{
    network::NodeId,
    packet::Packet,
};

pub type Message = String;
pub type MediaRef = String;
pub type FileId = u8;
pub type ServerId = NodeId;
pub type ClientId = NodeId;
pub type DroneId = NodeId;
pub type SessionId = u64;
pub type FloodId = u64;
pub type FragmentIndex = u64;
pub type UsingTimes = u64;  //to measure traffic of fragments in a path.

///packet sending status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotSentType{
    ToBeSent,
    Dropped,
    RoutingError,
    DroneDestination,
    BeenInWrongRecipient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Speaker{
    Me,
    HimOrHer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PacketStatus{
    Sent,                   //Successfully sent packet, that is with ack received
    NotSent(NotSentType),   //Include the packet not successfully sent, that is nack received
    InProgress,             //When we have no ack or nack confirmation
}

/// From controller to Server
#[derive(Debug, Clone)]
pub enum ServerCommand {
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>),
    ShortcutPacket(Packet),
}

///Server-Controller
pub enum ServerEvent {
}


#[derive(Debug)]
/// From controller to Client
pub enum ClientCommand {
    //Controller functions
    RemoveSender(NodeId),
    AddSender(NodeId, Sender<Packet>),
    SendMessageTo(ClientId, Message),  //if you order a client to send messages to another client you can do it
    RunUI,
    StartFlooding,
    AskTypeTo(ServerId),
    RequestListFile(ServerId),   //request the list of the file that the server has.
    RequestText(ServerId, FileId),  //the type File is alias of String, so we are requesting a Text in the File.
    RequestMedia(ServerId, MediaRef), //the type Media is alias of String, we are requesting the content referenced by the MediaRef.
    ShortcutPacket(Packet),
    GetKnownServers,
}


#[derive(Debug, Clone)]
pub enum ClientEvent {
    PacketSent(Packet),
    KnownServers(Vec<(NodeId, ServerType, bool)>),
}

//Queries (Client -> Server)
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Query {
    //Common-shared
    AskType,

    //To Communication Server
    RegisterClient(NodeId),
    UnregisterClient(NodeId),
    AskListClients,
    SendMessageTo(NodeId, Message),

    //To Content Server
    //(Text)
    AskListFiles,
    AskFile(FileId),   //changed to File (String)
    //(Media)
    AskMedia(String), // String is the reference found in the files
}

//Server -> Client
#[derive(Deserialize, Serialize, Debug)]
pub enum Response {
    //Common-shared
    ServerType(ServerType),

    //From Communication Server
    ClientRegistered,
    MessageFrom(NodeId, Message),
    ListClients(Vec<NodeId>),

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
#[derive(Deserialize, Serialize, Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub enum ServerType {
    Communication,
    Text,
    Media,
    Undefined,
}

impl Display for ServerType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ServerType::Communication => "Communication",
            ServerType::Text => "Text",
            ServerType::Media => "Media",
            ServerType::Undefined => "Undefined",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Serialize, Deserialize)]
pub enum ClientType {
    Chat,
    Web,
}
impl ClientType {
    // Returns an iterator over all possible client types
    pub fn iter() -> impl Iterator<Item = ClientType> {
        [
            ClientType::Chat,
            ClientType::Web,
        ]
            .into_iter()
    }
}

impl Display for ClientType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
