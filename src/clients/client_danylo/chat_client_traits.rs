use crossbeam_channel::Sender;

use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{FloodRequest, FloodResponse, Fragment, Nack, Packet},
};

use crate::general_use::{
    ClientCommand, ClientEvent, ClientId, FloodId, FragmentIndex, Message, Query, Response,
    ServerId, ServerType, SessionId, Node,
};

pub(super) trait PacketHandler {
    fn handle_packet(&mut self, packet: Packet);
    fn handle_ack(&mut self, fragment_index: FragmentIndex, session_id: SessionId);
    fn handle_nack(&mut self, nack: Nack, session_id: SessionId);
    fn update_topology_and_routes(&mut self, error_node: NodeId);
    fn find_route_to(&self, server_id: ServerId) -> Option<Vec<NodeId>>;
    fn update_message_route_and_resend(&mut self, fragment_index: FragmentIndex, session_id: SessionId);
    fn update_message_route(&mut self, session_id: &SessionId) -> Result<(), String>;
    fn handle_fragment(&mut self, fragment: Fragment, session_id: SessionId, server_id: ServerId);
    fn handle_flood_request(&mut self, flood_request: FloodRequest, session_id: SessionId);
    fn handle_flood_response(&mut self, flood_response: FloodResponse);
    fn update_topology(&mut self, path: &[Node]);
    fn update_routes_and_servers(&mut self, path: &[Node]);
}

pub(super) trait CommandHandler {
    fn handle_command(&mut self, command: ClientCommand);
    fn add_sender(&mut self, id: NodeId, sender: Sender<Packet>);
    fn remove_sender(&mut self, id: NodeId);
    fn send_known_servers(&mut self);
    fn discovery(&mut self);
    fn request_server_type(&mut self, server_id: ServerId);
    fn send_message_to(&mut self, to: ClientId, message: Message);
    fn request_to_register(&mut self, server_id: ServerId);
    fn request_clients_list(&mut self, server_id: ServerId);
    fn create_and_send_message(&mut self, query: Query, server_id: ServerId) -> Result<(), String>;
}

pub(super) trait ServerResponseHandler {
    fn handle_server_response(&mut self, response: Option<Response>, server_id: ServerId);
    fn handle_server_type(&mut self, server_id: ServerId, server_type: ServerType);
    fn handle_client_registered(&mut self, server_id: ServerId);
    fn handle_clients_list(&mut self, server_id: ServerId, clients: Vec<ClientId>);
    fn handle_message_from(&mut self, from: ClientId, message: Message);
}

pub(super) trait Senders {
    fn send_to_next_hop(&mut self, packet: Packet) -> Result<(), String>;
    fn send_ack(&mut self, fragment_index: FragmentIndex, session_id: SessionId, routing_header: SourceRoutingHeader);
    fn send_event(&self, event: ClientEvent);
    fn resend_fragment(&mut self, fragment_index: FragmentIndex, session_id: SessionId);
    fn resend_queries(&mut self);
}

pub(super) trait GeneratorId {
    fn generate_session_id(&mut self) -> SessionId;
    fn generate_flood_id(&mut self) -> FloodId;
    fn parse_id(&self, id: u64) -> u64;
}

pub(super) trait Reassembler {
    fn reassemble(&mut self, session_id: SessionId) -> Option<Response>;
}
