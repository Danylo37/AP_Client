use crossbeam_channel::Sender;
use log::{debug, error, info, warn};

use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{NodeType, Packet, FloodRequest}
};

use crate::general_use::{ClientCommand, ClientEvent, ClientId, Message, Query, ServerId, ServerType};
use super::{CommandHandler, ChatClientDanylo, PacketHandler, Senders, GeneratorId, MessageFragments};

impl CommandHandler for ChatClientDanylo {
    /// ###### Handles incoming commands.
    fn handle_command(&mut self, command: ClientCommand) {
        match command {
            ClientCommand::AddSender(id, sender) => {
                self.add_sender(id, sender);
            }
            ClientCommand::RemoveSender(id) => {
                self.remove_sender(id);
            }
            ClientCommand::ShortcutPacket(packet) => {
                info!("Client {}: Shortcut packet received from SC: {:?}", self.id, packet);
                self.handle_packet(packet);
            }
            ClientCommand::GetKnownServers => {
                self.send_known_servers()
            }
            ClientCommand::StartFlooding => {
                self.discovery()
            }
            ClientCommand::AskTypeTo(server_id) => {
                self.request_server_type(server_id)
            }
            ClientCommand::SendMessageTo(to, message) => {
                self.send_message_to(to, message)
            }
            ClientCommand::RegisterToServer(server_id) => {
                self.request_to_register(server_id)
            }
            ClientCommand::AskListClients(server_id) => {
                self.request_clients_list(server_id)
            }
            _ => {}
        }
    }

    /// ###### Adds a sender for specified node.
    fn add_sender(&mut self, id: NodeId, sender: Sender<Packet>) {
        self.packet_send.insert(id, sender);
        info!("Client {}: Added sender for node {}", self.id, id);
    }

    /// ###### Removes a sender for specified node.
    fn remove_sender(&mut self, id: NodeId) {
        self.packet_send.remove(&id);
        self.update_topology_and_routes(id);
        info!("Client {}: Removed sender for node {}", self.id, id);
    }

    /// ###### Handles the 'GetKnownServers' command.
    /// Sends the list of known servers to the simulation controller.
    fn send_known_servers(&mut self) {
        let servers: Vec<(ServerId, ServerType, bool)> = self
            .servers
            .iter()
            .map(|(&id, &server_type)| (
                id,
                server_type,
                *self.is_registered.get(&id).unwrap_or(&false)))
            .collect();

        self.send_event(ClientEvent::KnownServers(servers));
    }

    /// ###### Initiates the discovery process to find available servers and clients.
    /// Clears current data structures and sends a flood request to all neighbors.
    fn discovery(&mut self) {
        info!("Client {}: Starting discovery process", self.id);

        // Clear all current data structures related to topology.
        self.routes.clear();
        self.topology.clear();

        // Generate a new flood ID.
        let flood_id = self.generate_flood_id();
        self.flood_ids.push(flood_id);

        // Create a new flood request initialized with the generated flood ID, the current node's ID, and its type.
        let flood_request = FloodRequest::initialize(
            flood_id,
            self.id,
            NodeType::Client,
        );

        // Generate a new session ID.
        let session_id = self.generate_session_id();
        self.session_ids.push(session_id);

        // Create a new packet with the flood request and session ID.
        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            session_id,
            flood_request,
        );

        // Attempt to send the flood request to all neighbors.
        for sender in &self.packet_send {
            if let Err(err) = sender.1.send(packet.clone()) {
                error!("Client {}: Failed to send FloodRequest to the drone {}: {}", self.id, sender.0, err);
            } else {
                info!("Client {}: FloodRequest sent to the drone with id {}.", self.id, sender.0);

                // Send the 'PacketSent' event to the simulation controller.
                self.send_event(ClientEvent::PacketSent(packet.clone()));
            }
        }
    }

    /// ###### Requests the server type for a specified server.
    fn request_server_type(&mut self, server_id: ServerId) {
        debug!("Client {}: Requesting server type for server {}", self.id, server_id);

        let result = self.create_and_send_message(Query::AskType, server_id);

        match result {
            Ok(_) => {
                info!("Client {}: Request for server type sent successfully.", self.id);
            }
            Err(err) => {
                let error_string = format!("Client {}: Failed to send request for server type: {}", self.id, err);

                if err == "Topology is empty. Discovery started and the query will be resent" {
                    warn!("{}", error_string);
                } else {
                    error!("{}", error_string);
                }
            },
        }
    }

    /// ###### Sends a message to a specified client.
    /// Sends a message to the server that the client is connected to,
    /// which then forwards the message to the specified client.
    fn send_message_to(&mut self, to: ClientId, message: Message) {
        let option_server_id = self.clients.iter()
            .find(|(_, clients)| clients.contains(&to))
            .map(|(server_id, _)| *server_id);

        let server_id = match option_server_id {
            Some(id) => id,
            None => {
                error!("Client {}: Failed to send message: Client {} is not found", self.id, to);
                return;
            }
        };

        debug!("Client {}: Sending message to client {} via server {}", self.id, to, server_id);

        let result = self.create_and_send_message(Query::SendMessageTo(to, message.clone()), server_id);

        match result {
            Ok(_) => {
                info!("Client {}: Message sent successfully.", self.id);
                let chat = self.chats.entry(to).or_insert_with(Vec::new);
                chat.push((self.id, message));
            }
            Err(err) => {
                let error_string = format!("Client {}: Failed to send message: {}", self.id, err);

                if err == "Topology is empty. Discovery started and the query will be resent" {
                    warn!("{}", error_string);
                } else {
                    error!("{}", error_string);
                }
            },
        }
    }

    /// ###### Requests to register the client on a specified server.
    fn request_to_register(&mut self, server_id: ServerId) {
        if let Some(is_registered) = self.is_registered.get(&server_id) {
            if *is_registered {
                warn!("Client {}: Already registered on server {}", self.id, server_id);
                self.ui_response_send.send(crate::general_use::Response::Err("Already registered".to_string())).unwrap();
                return;
            }
        }

        debug!("Client {}: Requesting to register on server {}", self.id, server_id);

        let result = self.create_and_send_message(Query::RegisterClient(self.id), server_id);

        match result {
            Ok(_) => {
                info!("Client {}: Request to register sent successfully.", self.id);
            }
            Err(err) => {
                let error_string = format!("Client {}: Failed to send request to register: {}", self.id, err);

                if err == "Topology is empty. Discovery started and the query will be resent" {
                    warn!("{}", error_string);
                } else {
                    error!("{}", error_string);
                }
            },
        }
    }

    /// ###### Requests the list of clients from a specified server.
    fn request_clients_list(&mut self, server_id: ServerId) {
        debug!("Client {}: Requesting clients list from server {}", self.id, server_id);

        let result = self.create_and_send_message(Query::AskListClients, server_id);

        match result {
            Ok(_) => {
                info!("Client {}: Request for clients list sent successfully.", self.id);
            }
            Err(err) => {
                let error_string = format!("Client {}: Failed to send request for clients list: {}", self.id, err);

                if err == "Topology is empty. Discovery started and the query will be resent" {
                    warn!("{}", error_string);
                } else {
                    error!("{}", error_string);
                }
            },
        }
    }

    /// ###### Creates and sends a message to a specified server.
    /// Serializes the data, splits it into fragments, and sends the first fragment.
    fn create_and_send_message(&mut self, query: Query, server_id: ServerId) -> Result<(), String> {
        debug!("Client {}: Creating and sending message to server {}: {:?}", self.id, server_id, query);

        // Check if the topology is empty and start the discovery process if it is.
        if self.topology.is_empty() {
            self.discovery();
            self.queries_to_resend.push_back((server_id, query));
            return Err("Topology is empty. Discovery started and the query will be resent".to_string());
        }

        // Find a route to the server or use the cached route if available.
        let hops = if let Some(route) = self.routes.get(&server_id) {
            route.clone()
        } else if let Some(route) = self.find_route_to(server_id) {
            self.routes.insert(server_id, route.clone());
            route
        } else {
            return Err(format!("No routes to the server with id {server_id}"));
        };

        // Generate a new session ID.
        let session_id = self.generate_session_id();
        self.session_ids.push(session_id);

        // Create message (split the query into fragments) and send first fragment.
        let mut message = MessageFragments::new(session_id, hops);
        if message.create_message_of(query) {
            self.messages_to_send.insert(session_id, message.clone());
            self.send_to_next_hop(message.get_fragment_packet(0).unwrap())
        } else {
            Err("Failed to create message.".to_string())
        }
    }
}