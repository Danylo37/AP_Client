use std::collections::{HashMap, HashSet, VecDeque};

use crossbeam_channel::{select_biased, Receiver, Sender};
use log::{info, debug, warn, error};

use wg_2024::{
    network::{NodeId, SourceRoutingHeader},
    packet::{FloodRequest, FloodResponse, Fragment, Nack, NackType, NodeType, Packet, PacketType},
};

use crate::{
    general_use::{
        ClientCommand, ClientEvent, Message, Query, Response, ServerType,
        ClientId, ServerId, SessionId, FloodId, FragmentIndex
    },
    clients::Client
};
use super::MessageFragments;

pub type Node = (NodeId, NodeType);
pub type ChatHistory = Vec<(ClientId, Message)>;

pub struct ChatClientDanylo {
    // ID
    pub id: ClientId,                                                 // Client ID

    // Channels
    pub packet_send: HashMap<NodeId, Sender<Packet>>,                 // Neighbor's packet sender channels
    pub packet_recv: Receiver<Packet>,                                // Packet receiver channel
    pub controller_send: Sender<ClientEvent>,                         // Event sender channel
    pub controller_recv: Receiver<ClientCommand>,                     // Command receiver channel
    pub ui_response_send: Sender<Response>,                           // UI response sender channel

    // Servers and clients
    pub servers: HashMap<ServerId, ServerType>,                       // IDs and types of the available servers
    pub is_registered: HashMap<ServerId, bool>,                       // Registration status on servers
    pub clients: HashMap<ServerId, Vec<ClientId>>,                    // Available clients on different servers

    // Used IDs
    pub session_id_counter: SessionId,                                // Counter for session IDs
    pub flood_id_counter: FloodId,                                    // Counter for flood IDs
    pub session_ids: Vec<SessionId>,                                  // Used session IDs
    pub flood_ids: Vec<FloodId>,                                      // Used flood IDs

    // Network
    pub topology: HashMap<NodeId, HashSet<NodeId>>,                   // Nodes and their neighbours
    pub routes: HashMap<ServerId, Vec<NodeId>>,                       // Routes to the servers

    // Message queues
    pub messages_to_send: HashMap<SessionId, MessageFragments>,       // Queue of messages to be sent for different sessions
    pub fragments_to_reassemble: HashMap<SessionId, Vec<Fragment>>,   // Queue of fragments to be reassembled for different sessions
    pub queries_to_resend: VecDeque<(ServerId, Query)>,               // Queue of queries to resend

    // Chats
    pub chats: HashMap<ClientId, ChatHistory>,                        // Chat histories with other clients
}

impl Client for ChatClientDanylo {
    fn new(
        id: NodeId,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
        ui_response_send: Sender<Response>,
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
                        self.handle_command(command);
                    }
                },
                recv(self.packet_recv) -> packet_res => {
                    if let Ok(packet) = packet_res {
                        self.handle_packet(packet);
                    }
                },
            }
        }
    }
}

impl ChatClientDanylo {
    /// ###### Handles incoming packets and delegates them to the appropriate handler based on the packet type.
    pub(crate) fn handle_packet(&mut self, packet: Packet) {
        debug!("Client {}: Handling packet: {:?}", self.id, packet);

        match packet.pack_type.clone() {
            PacketType::Ack(ack) => self.handle_ack(ack.fragment_index, packet.session_id),
            PacketType::Nack(nack) => self.handle_nack(nack, packet.session_id),
            PacketType::MsgFragment(fragment) => {
                // Send acknowledgment for the received fragment
                self.send_ack(fragment.fragment_index, packet.session_id, packet.routing_header.clone());

                // Get the server ID from the routing header and handle the fragment
                let server_id = packet.routing_header.hops.first().unwrap();
                self.handle_fragment(fragment, packet.session_id, *server_id)
            }
            PacketType::FloodRequest(flood_request) => self.handle_flood_request(flood_request, packet.session_id),
            PacketType::FloodResponse(flood_response) => self.handle_flood_response(flood_response),
        }
    }

    /// ###### Handles incoming commands.
    pub(crate) fn handle_command(&mut self, command: ClientCommand) {
        debug!("Client {}: Handling command: {:?}", self.id, command);

        match command {
            ClientCommand::AddSender(id, sender) => {
                self.packet_send.insert(id, sender);
                info!("Client {}: Added sender for node {}", self.id, id);
            }
            ClientCommand::RemoveSender(id) => {
                self.packet_send.remove(&id);
                info!("Client {}: Removed sender for node {}", self.id, id);
            }
            ClientCommand::ShortcutPacket(packet) => {
                info!("Client {}: Shortcut packet received from SC: {:?}", self.id, packet);
                self.handle_packet(packet);
            }
            ClientCommand::GetKnownServers => {
                self.handle_get_known_servers()
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

    /// ###### Handles the 'GetKnownServers' command.
    /// Sends the list of known servers to the simulation controller.
    fn handle_get_known_servers(&mut self) {
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

    /// ###### Handles the acknowledgment (ACK) for a given session and fragment.
    /// Processes the acknowledgment for a specific fragment in a session.
    /// If there are more fragments to send, it sends the next fragment.
    /// If all fragments are acknowledged, it removes the message from queue.
    fn handle_ack(&mut self, fragment_index: FragmentIndex, session_id: SessionId) {
        debug!("Client {}: Handling ACK for session {} and fragment {}", self.id, session_id, fragment_index);

        // Retrieve the message fragments for the given session.
        let message = self.messages_to_send.get_mut(&session_id).unwrap();

        // Check if there is a next fragment to send.
        if let Some(next_fragment) = message.get_fragment_packet((fragment_index + 1) as usize) {
            // Prepare and send the next fragment if available.
            message.increment_last_index();
            match self.send_to_next_hop(next_fragment) {
                Ok(_) => info!("Client {}: Sent next fragment for session {}", self.id, session_id),
                Err(err) => error!("Client {}: Failed to send next fragment for session {}: {}", self.id, session_id, err),
            }
        } else {
            // All fragments are acknowledged; remove the message from queue.
            self.messages_to_send.remove(&session_id);
            info!("Client {}: All fragments acknowledged for session {}", self.id, session_id);
        }
    }

    /// ###### Handles the negative acknowledgment (NACK) for a given session.
    /// Processes the NACK for a specific session and takes appropriate action based on the NACK type.
    fn handle_nack(&mut self, nack: Nack, session_id: SessionId) {
        warn!("Client {}: Handling NACK for session {}: {:?}", self.id, session_id, nack);

        match nack.nack_type {
            NackType::ErrorInRouting(id) => {
                self.update_topology_and_routes(id);
                self.update_message_route_and_resend(nack.fragment_index, session_id);
            }
            NackType::DestinationIsDrone
            | NackType::UnexpectedRecipient(_) => {
                // Impossible errors
                self.update_message_route_and_resend(nack.fragment_index, session_id);
            }
            NackType::Dropped => {
                self.resend_fragment(nack.fragment_index, session_id);
            }
        }
    }

    /// ###### Updates the message route and resends the fragment.
    fn update_message_route_and_resend(&mut self, fragment_index: FragmentIndex, session_id: SessionId) {
        match self.update_message_route(&session_id) {
            Ok(_) => {
                self.resend_fragment(fragment_index, session_id);
            }
            Err(err) => {
                error!("Client {}: Impossible to resend fragment: {}", self.id, err);
            }
        }
    }

    /// ###### Updates the network topology and routes.
    /// Removes the node that caused the error from the topology and routes.
    /// Finds new routes for the servers that need them.
    fn update_topology_and_routes(&mut self, error_node: NodeId) {
        // Remove the node that caused the error from the topology.
        for (_, neighbors) in self.topology.iter_mut() {
            neighbors.remove(&error_node);
        }
        self.topology.remove(&error_node);
        info!("Client {}: Removed node {} from the topology", self.id, error_node);

        // Replace the paths that contain the node that caused the error with an empty vector.
        for route in self.routes.values_mut() {
            if route.contains(&error_node) {
                *route = Vec::new();
            }
        }
        info!("Client {}: Routes with node {} cleared", self.id, error_node);

        // Collect server IDs that need new routes.
        let servers_to_update: Vec<ServerId> = self
            .routes
            .iter()
            .filter(|(_, path)| path.is_empty())
            .map(|(server_id, _)| *server_id)
            .collect();

        // Find new routes for the collected server IDs.
        for server_id in servers_to_update {
            if let Some(new_path) = self.find_route_to(server_id) {
                if let Some(path) = self.routes.get_mut(&server_id) {
                    *path = new_path;
                    info!("Client {}: Found new route to the server {}: {:?}", self.id, server_id, path);
                }
            } else {
                warn!("Client {}: No route found to the server {}", self.id, server_id);
            }
        }
    }

    /// ###### Updates the route for the message with the specified session ID.
    /// If a new route is found, it updates the message with the new route.
    /// If no route is found, it returns an error message.
    fn update_message_route(&mut self, session_id: &SessionId) -> Result<(), String> {
        let message = self.messages_to_send.get_mut(session_id).unwrap();
        let dest_id = message.get_route().last().unwrap();

        if let Some(new_route) = self.routes.get(dest_id) {
            message.update_route(new_route.clone());
            Ok(())
        } else {
            Err(format!("No routes to the server {}", dest_id))
        }
    }

    /// ###### Finds a route from the current node to the specified server using breadth-first search.
    ///
    /// This method explores the network topology starting from the current node, and returns the shortest path
    /// (in terms of hops) to the specified server if one exists. It uses a queue to explore nodes level by level,
    /// ensuring that the first valid path found is the shortest. If no path is found, it returns `None`.
    fn find_route_to(&self, server_id: ServerId) -> Option<Vec<NodeId>> {
        // Initialize a queue for breadth-first search and a set to track visited nodes.
        let mut queue: VecDeque<(NodeId, Vec<NodeId>)> = VecDeque::new();
        let mut visited: HashSet<NodeId> = HashSet::new();

        // Start from the current node with an initial path containing just the current node.
        queue.push_back((self.id, vec![self.id]));

        // Perform breadth-first search.
        while let Some((current, path)) = queue.pop_front() {
            // If the destination node is reached, return the path.
            if current == server_id {
                return Some(path);
            }

            // Mark the current node as visited.
            visited.insert(current);

            // Explore the neighbors of the current node.
            if let Some(neighbors) = self.topology.get(&current) {
                for &neighbor in neighbors {
                    // Only visit unvisited neighbors.
                    if !visited.contains(&neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor); // Extend the path to include the neighbor.
                        queue.push_back((neighbor, new_path)); // Add the neighbor to the queue.
                    }
                }
            }
        }
        None    // Return None if no path to the server is found.
    }

    /// ###### Resends the fragment for the specified session.
    /// Retrieves the message and resends the fragment with the specified index.
    fn resend_fragment(&mut self, fragment_index: FragmentIndex, session_id: SessionId) {
        debug!("Client {}: Resending fragment {} for session {}", self.id, fragment_index, session_id);

        let message = self.messages_to_send.get(&session_id).unwrap();
        let packet = message.get_fragment_packet(fragment_index as usize).unwrap();
        match self.send_to_next_hop(packet) {
            Ok(_) =>
                info!("Client {}: Resent fragment {} for session {}", self.id, fragment_index, session_id),
            Err(err) =>
                error!("Client {}: Failed to resend fragment {} for session {}: {}", self.id, fragment_index, session_id, err),
        }
    }

    /// ###### Handles a received message fragment.
    /// Adds the fragment to the collection for the session and checks if it is the last fragment.
    /// If it is the last fragment, reassembles the message and processes the server response.
    fn handle_fragment(&mut self, fragment: Fragment, session_id: SessionId, server_id: ServerId) {
        debug!("Client {}: Handling fragment for session {}: {:?}", self.id, session_id, fragment);

        // Retrieve or create a vector to store fragments for the session.
        let fragments = self.fragments_to_reassemble.entry(session_id).or_insert_with(Vec::new);

        // Add the current fragment to the collection.
        fragments.push(fragment.clone());

        // Check if the current fragment is the last one in the sequence.
        if fragment.fragment_index == fragment.total_n_fragments - 1 {
            // Reassemble the fragments into a complete message and process it.
            let message = self.reassemble(session_id);
            self.handle_server_response(message, server_id);
        }
    }

    /// ###### Handles the server response.
    /// Processes the server response based on its type and takes appropriate actions.
    fn handle_server_response(&mut self, response: Option<Response>, server_id: ServerId) {
        debug!("Client {}: Handling server response for server {}: {:?}", self.id, server_id, response);

        if let Some(response) = response {
            match response {
                Response::ServerType(server_type) => {
                    self.handle_server_type(server_id, server_type);
                },
                Response::ClientRegistered => {
                    self.handle_client_registered(server_id);
                }
                Response::ListClients(clients) => {
                    self.handle_clients_list(server_id, clients);
                }
                Response::MessageFrom(from, message) => {
                    info!("Client {}: New message from {}: {:?}", self.id, from, &message);

                    self.ui_response_send.send(Response::MessageFrom(from, message.clone())).unwrap();

                    let chat = self.chats.entry(from).or_insert_with(Vec::new);
                    chat.push((from, message));
                }
                Response::Err(error) =>
                    error!("Client {}: Error received from server {}: {:?}", self.id, server_id, error),
                _ => {}
            }
        }
    }

    /// ###### Handles the server type response.
    /// Updates the server type in the `servers` map and sets the registration status if the server is of type `Communication`
    /// and marks the response as received.
    fn handle_server_type(&mut self, server_id: ServerId, server_type: ServerType) {
        info!("Client {}: Server type received successfully.", self.id);

        // Insert the server type into the servers map.
        self.servers.insert(server_id, server_type);

        // If the server is of type Communication, set the registration status to false.
        if server_type == ServerType::Communication {
            self.is_registered.insert(server_id, false);
        }

        self.ui_response_send.send(Response::ServerType(server_type)).unwrap();
    }

    /// ###### Handles the client registration response.
    /// Updates the registration status for the specified server and marks the response as received.
    fn handle_client_registered(&mut self, server_id: ServerId) {
        info!("Client {}: Client registered successfully.", self.id);

        self.is_registered.insert(server_id, true);

        self.ui_response_send.send(Response::ClientRegistered).unwrap();
    }

    /// ###### Handles the list of clients received from the server.
    /// Updates the list of available clients.
    fn handle_clients_list(&mut self, server_id: ServerId, mut clients: Vec<ClientId>) {
        info!("Client {}: List of clients received successfully.", self.id);

        self.ui_response_send.send(Response::ListClients(clients.clone())).unwrap();

        // Remove self id from the clients list if it exists
        clients.retain(|&client_id| client_id != self.id);

        self.clients.insert(server_id, clients);

    }

    /// ###### Sends an acknowledgment (ACK) for a received fragment.
    /// Creates an ACK packet and sends it to the next hop.
    /// Logs the success or failure of the send operation.
    fn send_ack(&mut self, fragment_index: FragmentIndex, session_id: SessionId, mut routing_header: SourceRoutingHeader) {
        // Reverse the routing header and reset the hop index.
        routing_header.reverse();
        routing_header.reset_hop_index();

        let ack = Packet::new_ack(routing_header, session_id, fragment_index);

        // Attempt to send the ACK packet to the next hop.
        match self.send_to_next_hop(ack) {
            Ok(_) => {
                info!("Client {}: ACK sent successfully for session {} and fragment {}", self.id, session_id, fragment_index);
            }
            Err(err) => {
                error!("Client {}: Failed to send ACK for session {} and fragment {}: {}", self.id, session_id, fragment_index, err);
            }
        };
    }

    /// ###### Handles a flood request by adding the client to the path trace and generating a response.
    fn handle_flood_request(&mut self, mut flood_request: FloodRequest, session_id: SessionId) {
        debug!("Client {}: Handling flood request for session {}: {:?}", self.id, session_id, flood_request);

        // Add client to the flood request's path trace.
        flood_request.increment(self.id, NodeType::Client);

        // Generate a response for the flood request.
        let response = flood_request.generate_response(session_id);

        // Send the response to the next hop.
        match self.send_to_next_hop(response) {
            Ok(_) => info!("Client {}: FloodResponse sent successfully.", self.id),
            Err(err) => error!("Client {}: Error sending FloodResponse: {}", self.id, err),
        }
    }

    /// ###### Sends the packet to the next hop in the route.
    ///
    /// Attempts to send the packet to the next hop in the route.
    /// If the packet is successfully sent, it returns `Ok(())`.
    /// If an error occurs during the send operation, it returns an error message.
    fn send_to_next_hop(&mut self, mut packet: Packet) -> Result<(), String> {
        // Attempt to retrieve the next hop ID from the routing header.
        // If it is missing, return an error as there is no valid destination to send the packet to.
        let Some(next_hop_id) = packet.routing_header.next_hop() else {
            return Err("No next hop in the routing header.".to_string());
        };

        // Attempt to find the sender for the next hop.
        let Some(sender) = self.packet_send.get(&next_hop_id) else {
            return Err("No sender to the next hop.".to_string());
        };

        // Increment the hop index in the routing header.
        packet.routing_header.increase_hop_index();

        debug!("Client {}: Sending packet to next hop: {:?}", self.id, packet);
        // Attempt to send the packet to the next hop.
        if sender.send(packet.clone()).is_err() {
            return Err("Error sending packet to next hop.".to_string());
        } else {
            info!("Client {}: Packet sent to next hop: {}", self.id, next_hop_id);
        }

        // Send the 'PacketSent' event to the simulation controller
        self.send_event(ClientEvent::PacketSent(packet));

        Ok(())
    }

    /// ###### Handles the flood response by updating routes and topology.
    ///
    /// This function processes the received `FloodResponse` by updating the routes and servers
    /// based on the path trace provided in the response. It also updates the network topology
    /// with the new path information and updates the time of the last response.
    fn handle_flood_response(&mut self, flood_response: FloodResponse) {
        debug!("Client {}: Handling flood response: {:?}", self.id, flood_response);

        let path = &flood_response.path_trace;

        self.update_topology(path);
        self.update_routes_and_servers(path);

    }

    /// ###### Updates the network topology based on the provided path.
    /// Adds connections between nodes in both directions.
    fn update_topology(&mut self, path: &[Node]) {
        for i in 0..path.len() - 1 {
            let current = path[i].0;
            let next = path[i + 1].0;

            // Add the connection between the current and next node in both directions.
            self.topology
                .entry(current)
                .or_insert_with(HashSet::new)
                .insert(next);

            self.topology
                .entry(next)
                .or_insert_with(HashSet::new)
                .insert(current);
        }
        debug!("Client {}: Updated topology with path: {:?}", self.id, path);
    }

    /// ###### Updates the routes and servers based on the provided path.
    /// If the path leads to a server, it updates the routing table and the servers list.
    /// If there are queries waiting for the route to the server, it resends them.
    fn update_routes_and_servers(&mut self, path: &[Node]) {
        if let Some((id, NodeType::Server)) = path.last() {
            if self
                .routes
                .get(id)
                .map_or(true, |prev_path| prev_path.len() > path.len())
            {
                // Add the server to the servers list with an undefined type if it is not already present.
                if !self.servers.contains_key(id) {
                    self.servers.insert(*id, ServerType::Undefined);
                }

                // Update the routing table with the new, shorter path.
                self.routes.insert(
                    *id,
                    path.iter().map(|entry| entry.0.clone()).collect(),
                );
                info!("Client {}: Updated route to server {}: {:?}", self.id, id, path);

                // Resend queries that were waiting for the route to the server.
                if !self.queries_to_resend.is_empty() {
                    self.resend_queries();
                }
            }
        }
    }

    /// ###### Resends queries that were waiting for the route to the server.
    /// Iterates over the queries to resend and sends them to the corresponding servers.
    /// If the server is not in the routing table, the query is not resent.
    /// If the query is successfully sent, it is removed from the queue.
    fn resend_queries(&mut self) {
        let queries = self.queries_to_resend.clone();

        for (server_id, query) in queries {

            if !self.routes.contains_key(&server_id) {
                return;
            }

            match self.create_and_send_message(query.clone(), server_id) {
                Ok(_) => {
                    info!("Client {}: Query {:?} resent successfully", self.id, query);
                    self.queries_to_resend.pop_front();
                }
                Err(err) => {
                    error!("Client {}: Failed to resend query {:?}: {}",
                        self.id, query, err);
                }
            };
        }
    }

    /// ###### Initiates the discovery process to find available servers and clients.
    /// Clears current data structures and sends a flood request to all neighbors.
    pub fn discovery(&mut self) {
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
            if let Err(_) = sender.1.send(packet.clone()) {
                error!("Client {}: Failed to send FloodRequest to the drone {}.", self.id, sender.0);
            } else {
                info!("Client {}: FloodRequest sent to the drone with id {}.", self.id, sender.0);

                // Send the 'PacketSent' event to the simulation controller.
                self.send_event(ClientEvent::PacketSent(packet.clone()));
            }
        }
    }

    /// ###### Generates a new session ID.
    fn generate_session_id(&mut self) -> SessionId {
        self.session_id_counter += 1;
        let next_session_id: SessionId = self.session_id_counter;
        self.parse_id(next_session_id)
    }

    /// ###### Generates a new flood ID.
    fn generate_flood_id(&mut self) -> FloodId {
        self.flood_id_counter += 1;
        let next_flood_id: FloodId = self.flood_id_counter;
        self.parse_id(next_flood_id)
    }

    /// ###### Parses the ID by concatenating the client ID and the provided ID.
    fn parse_id(&self, id: u64) -> u64 {
        format!("{}{}", self.id, id)
            .parse()
            .unwrap()
    }

    /// ###### Requests the type of specified server.
    /// Sends a query to the server and waits for a response.
    pub fn request_server_type(&mut self, server_id: ServerId) {
        info!("Client {}: Requesting server type for server {}", self.id, server_id);

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

    /// ###### Requests to register the client on a specified server.
    /// Sends a registration query to the server and waits for a response.
    pub fn request_to_register(&mut self, server_id: ServerId) {
        if let Some(is_registered) = self.is_registered.get(&server_id) {
            if *is_registered {
                info!("Client {}: Already registered on server {}", self.id, server_id);
                self.ui_response_send.send(Response::Err("Already registered".to_string())).unwrap();
                return;
            }
        }

        info!("Client {}: Requesting to register on server {}", self.id, server_id);

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
    /// Sends a query to the server and waits for a response.
    pub fn request_clients_list(&mut self, server_id: ServerId) {
        info!("Client {}: Requesting clients list from server {}", self.id, server_id);

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

    /// ###### Sends a message to a specified client via a specified server.
    /// Sends the message and waits for a response.
    pub fn send_message_to(&mut self, to: ClientId, message: Message) {
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

        info!("Client {}: Sending message to client {} via server {}", self.id, to, server_id);

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

    /// ###### Reassembles the fragments for a given session into a complete message.
    /// Returns the reassembled message or an error if reassembly fails.
    fn reassemble(&mut self, session_id: SessionId) -> Option<Response> {
        debug!("Client {}: Reassembling message for session {}", self.id, session_id);

        // Retrieve the fragments for the given session.
        let fragments = match self.fragments_to_reassemble.get_mut(&session_id) {
            Some(fragments) => fragments,
            None => {
                error!("Client {}: No fragments found for session {}", self.id, session_id);
                return None;
            },
        };

        // Ensure all fragments belong to the same message by checking the total number of fragments.
        let total_n_fragments = match fragments.first() {
            Some(first) => first.total_n_fragments,
            None => {
                error!("Client {}: Fragment list is empty for session {}", self.id, session_id);
                return None;
            },
        };

        // Check if the number of fragments matches the expected total.
        if fragments.len() as u64 != total_n_fragments {
            error!(
                "Client {}: Incorrect number of fragments for session {}: expected {}, got {}",
                self.id,
                session_id,
                total_n_fragments,
                fragments.len()
            );
            return None;
        }

        // Collect data from all fragments.
        let mut result = Vec::new();
        for fragment in fragments {
            result.extend_from_slice(&fragment.data[..fragment.length as usize]);
        }

        // Convert the collected data into a string.
        let reassembled_string = match String::from_utf8(result) {
            Ok(string) => string,
            Err(err) => {
                error!(
                    "Client {}: Failed to convert data to string for session {}: {}",
                    self.id, session_id, err
                );
                return None;
            },
        };

        // Attempt to deserialize the string into an object.
        match serde_json::from_str(&reassembled_string) {
            Ok(deserialized) => Some(deserialized),
            Err(err) => {
                error!(
                    "Client {}: Failed to deserialize JSON for session {}: {}",
                    self.id, session_id, err
                );
                None
            },
        }
    }

    /// ###### Sends an event to the simulation controller.
    /// Logs the success or failure of the send operation.
    fn send_event(&self, event: ClientEvent) {
        let result = self.controller_send.send(event.clone());
        let event_name = match event {
            ClientEvent::PacketSent(_) => "PacketSent",
            ClientEvent::KnownServers(_) => "KnownServers",
        };

        match result {
            Ok(_) => info!("Client {}: Sent '{}' event to controller", self.id, event_name),
            Err(_) => error!("Client {}: Error sending '{}' event to controller", self.id, event_name),
        }
    }
}
