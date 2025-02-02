use crate::clients::client_chen::{ClientChen, SpecificInfo};
use crate::clients::client_chen::general_client_traits::*;
use crate::clients::client_chen::prelude::*;
impl CommunicationTools for ClientChen{
    fn get_discovered_servers_from_topology(&mut self) -> HashSet<ServerId> {
        self.network_info.topology.iter()
            .filter_map(|(&node_id, node_info)| {
                if let SpecificInfo::ServerInfo(_) = &node_info.specific_info {
                    Some(node_id) // Ensure node_id can be converted to ServerId
                } else {
                    None
                }
            })
            .collect()
    }
    fn get_edge_nodes_from_topology(&mut self) -> HashSet<NodeId> {
        self.network_info.topology.iter()
            .filter_map(|(&node_id, node_info)| {
                match &node_info.specific_info {
                    SpecificInfo::ServerInfo(_) | SpecificInfo::ClientInfo(_) => Some(node_id),
                    _ => None,
                }
            })
            .collect()
    }

}