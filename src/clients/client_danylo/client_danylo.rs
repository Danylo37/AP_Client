use std::collections::{HashMap, HashSet, VecDeque};

use crossbeam_channel::{select_biased, Receiver, Sender};
use log::info;

use wg_2024::{
    network::NodeId,
    packet::{Fragment, Packet},
};

use crate::{
    general_use::{
        ClientCommand, ClientEvent, Query, ServerType, ClientId, ServerId, SessionId, FloodId, ChatHistory
    },
    clients::Client
};
use super::{PacketHandler, CommandHandler, MessageFragments};

pub struct ChatClientDanylo {
    // ID
    pub(super) id: ClientId,                                                 // Client ID

    // Channels
    pub(super) packet_send: HashMap<NodeId, Sender<Packet>>,                 // Neighbor's packet sender channels
    pub(super) packet_recv: Receiver<Packet>,                                // Packet receiver channel
    pub(super) controller_send: Sender<ClientEvent>,                         // Event sender channel
    pub(super) controller_recv: Receiver<ClientCommand>,                     // Command receiver channel

    pub ui_response_send: Sender<crate::general_use::Response>,

    // Servers and clients
    pub(super) servers: HashMap<ServerId, ServerType>,                       // IDs and types of the available servers
    pub(super) is_registered: HashMap<ServerId, bool>,                       // Registration status on servers
    pub(super) clients: HashMap<ServerId, Vec<ClientId>>,                    // Available clients on different servers

    // Used IDs
    pub(super) session_id_counter: SessionId,                                // Counter for session IDs
    pub(super) flood_id_counter: FloodId,                                    // Counter for flood IDs
    pub(super) session_ids: Vec<SessionId>,                                  // Used session IDs
    pub(super) flood_ids: Vec<FloodId>,                                      // Used flood IDs

    // Network
    pub(super) topology: HashMap<NodeId, HashSet<NodeId>>,                   // Nodes and their neighbours
    pub(super) routes: HashMap<ServerId, Vec<NodeId>>,                       // Routes to the servers

    // Message queues
    pub(super) messages_to_send: HashMap<SessionId, MessageFragments>,       // Queue of messages to be sent for different sessions
    pub(super) fragments_to_reassemble: HashMap<SessionId, Vec<Fragment>>,   // Queue of fragments to be reassembled for different sessions
    pub(super) queries_to_resend: VecDeque<(ServerId, Query)>,               // Queue of queries to resend

    // Chats
    pub(super) chats: HashMap<ClientId, ChatHistory>,                        // Chat histories with other clients
}

impl Client for ChatClientDanylo {
    fn new(
        id: NodeId,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
        ui_response_send: Sender<crate::general_use::Response>,
    ) -> Self {
        info!("Starting ChatClientDanylo with ID: {}", id);
        Self {
            id,
            packet_send,
            packet_recv,
            controller_send,
            controller_recv,
            ui_response_send,
            servers: HashMap::new(),
            is_registered: HashMap::new(),
            clients: HashMap::new(),
            session_id_counter: 0,
            flood_id_counter: 0,
            session_ids: Vec::new(),
            flood_ids: Vec::new(),
            topology: HashMap::new(),
            routes: HashMap::new(),
            messages_to_send: HashMap::new(),
            fragments_to_reassemble: HashMap::new(),
            queries_to_resend: VecDeque::new(),
            chats: HashMap::new(),
        }
    }

    fn run(&mut self) {
        info!("Running ChatClientDanylo with ID: {}", self.id);
        loop {
            select_biased! {
                recv(self.controller_recv) -> command_res => {
                    if let Ok(command) = command_res {
                        info!("Client {}: Received command: {:?}", self.id, command);
                        self.handle_command(command);
                    }
                },
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        info!("Client {}: Received packet: {:?}", self.id, packet);
                        self.handle_packet(packet);
                    }
                },
            }
        }
    }
}
