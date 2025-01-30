use std::collections::{HashSet, VecDeque};
use log::{debug, error, info};

use wg_2024::{
    packet::{Fragment, Nack, NackType, Packet, PacketType, FloodRequest, NodeType, FloodResponse},
    network::NodeId,
};

use crate::general_use::{FragmentIndex, ServerId, ServerType, SessionId, Node};
use super::{PacketHandler, ChatClientDanylo, Senders, ServerResponseHandler, Reassembler};

impl PacketHandler for ChatClientDanylo {
    /// ###### Handles incoming packets and delegates them to the appropriate handler based on the packet type.
    fn handle_packet(&mut self, packet: Packet) {
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
        debug!("Client {}: Handling NACK for session {}: {:?}", self.id, session_id, nack);

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

    /// ###### Updates the network topology and routes.
    /// Removes the node that caused the error from the topology and routes.
    /// Finds new routes for the servers that need them.
    fn update_topology_and_routes(&mut self, error_node: NodeId) {
        // Remove the node that caused the error from the topology.
        for (_, neighbors) in self.topology.iter_mut() {
            neighbors.remove(&error_node);
        }
        self.topology.remove(&error_node);
        debug!("Client {}: Removed node {} from the topology", self.id, error_node);

        // Replace the paths that contain the node that caused the error with an empty vector.
        for route in self.routes.values_mut() {
            if route.contains(&error_node) {
                *route = Vec::new();
            }
        }
        debug!("Client {}: Routes with node {} cleared", self.id, error_node);

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
                    debug!("Client {}: Found new route to the server {}: {:?}", self.id, server_id, path);
                }
            } else {
                error!("Client {}: No route found to the server {}", self.id, server_id);
            }
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

    /// ###### Updates the message route and resends the fragment if possible.
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

    /// ###### Handles received message fragment.
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

    /// ###### Handles the flood response by updating routes and topology.
    ///
    /// This function processes the received `FloodResponse` by updating the routes and servers
    /// based on the path trace provided in the response.
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
        info!("Client {}: Updated topology with path: {:?}", self.id, path);
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
}