use crate::clients::client_chen::{ClientChen, NodeInfo, PacketCreator, Router, Sending, SpecificInfo};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;

impl Router for ClientChen {
    ///main method of for discovering the routing
    fn do_flooding(&mut self) {
        // New ids for the flood and new session because of the flood response packet
        self.status.flood_id += 1;
        self.status.session_id += 1;

        self.communication.routing_table.clear();
        self.network_info.topology.clear();

        // Initialize the flood request with the current flood_id, id, and node type
        let flood_request = FloodRequest::initialize(self.status.flood_id, self.metadata.node_id, NodeType::Client);

        // Prepare the packet with the current session_id and flood_request
        let packet = Packet::new_flood_request(
            SourceRoutingHeader::empty_route(),
            self.status.session_id,
            flood_request,
        );

        // Collect the connected node IDs into a temporary vector
        let connected_nodes: Vec<_> = self.communication.connected_nodes_ids.iter().cloned().collect();

        // Send the packet to each connected node
        for &node_id in &connected_nodes {
            self.send_packet_to_connected_node(node_id, packet.clone()); // Assuming `send_packet_to_connected_node` takes a cloned packet
        }
    }

    fn update_routing_for_server(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId, NodeType)>) {
        //ask some necessary queries to the server, to get the communicable clients as early as possible.
        // useful for the chat clients
        // self.storage.irresolute_path_traces.remove(&destination_id);
        let hops = self.get_hops_from_path_trace(path_trace);

        //this will be done from the controller.
        self.send_query(destination_id, Query::AskType);
        self.communication.routing_table
            .entry(destination_id)
            .or_insert_with(HashMap::new)
            .insert(hops, 0);   //using times is 0, because we do the flooding when all the routing table is cleared.
        info!("Successfully updated routing table for server {}", destination_id);
        info!("The routing table is: {:?}", self.communication.routing_table);
    }
    fn update_routing_for_client(&mut self, destination_id: NodeId, path_trace: Vec<(NodeId, NodeType)>) {
        let hops = self.get_hops_from_path_trace(path_trace.clone());
        self.communication.routing_table
            .entry(destination_id)
            .or_insert_with(HashMap::new)
            .insert(hops, 0);   //using times is 0, because we do the flooding when all the routing table is cleared.
        info!("Successfully updated routing table for client {}", destination_id);
        info!("The routing table is: {:?}", self.communication.routing_table);
        //this is maybe for the chat clients but right now it is not use ful
        /*
        if self.check_if_exists_registered_communication_server_intermediary_in_route(hops) {
            //if exists a registered communication server that it is an intermediary between two clients then it will be ok
            self.storage.irresolute_path_traces.remove(&destination_id);
            let hops = self.get_hops_from_path_trace(path_trace);
            self.communication.routing_table
                .entry(destination_id)
                .or_insert_with(HashMap::new)
                .insert(hops, 0);
        } else {
            //the path_trace is still irresolute, that's a clever way
        }*/
    }


    //only for chat clients
    /*
    fn update_routing_checking_status(&mut self) {
        let mut updates = Vec::new();
        for (&destination_id, traces) in &self.storage.irresolute_path_traces {
            if let Some((_, destination_type)) = traces.last() {
                updates.push((destination_id, *destination_type, traces.clone()));
            } else {
                warn!("No traces found for destination: {:?}", destination_id);
            }
        }

        // Apply the updates
        for (destination_id, destination_type, path_trace) in updates {
            match destination_type {
                NodeType::Server => {
                    self.update_routing_for_server(destination_id, path_trace);
                }
                NodeType::Client => {
                    self.update_routing_for_client(destination_id, path_trace);
                }
                _ => {
                    // Handle other cases (e.g., NodeType::Drone)
                    warn!("Unhandled node type: {:?}", destination_type);
                }
            }
        }
    }
*/
    ///auxiliary function
    fn check_if_exists_registered_communication_server_intermediary_in_route(&mut self, route: Vec<NodeId>) -> bool {
        for &server_id in self.communication.registered_communication_servers.keys() {
            if route.contains(&server_id) {
                return true;
            }
        }
        false
    }

    fn check_if_exists_route_contains_server(&mut self, server_id: ServerId, destination_id: ClientId) -> bool {
        for route in self.communication.routing_table.get(&destination_id).unwrap() {
            if route.0.contains(&server_id) {
                return true;
            }
        }
        false
    }

    fn get_flood_response_initiator(&mut self, flood_response: FloodResponse) -> NodeId {
        flood_response.path_trace.last().map(|(id, _)| *id).unwrap()
    }

    fn update_topology_entry_for_server(&mut self, initiator_id: NodeId, server_type: ServerType) {
        if let SpecificInfo::ServerInfo(server_info) = &mut self
            .network_info
            .topology
            .entry(initiator_id)
            .or_insert(NodeInfo::default()) // Use `or_insert` here
            .specific_info
        {
            server_info.server_type = server_type;
        }
    }

}



