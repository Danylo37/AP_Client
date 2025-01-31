use crate::clients::client_chen::{ClientChen, PacketResponseHandler, Router, Sending};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;

impl PacketResponseHandler for ClientChen {
    fn handle_ack(&mut self, ack_packet: Packet, ack: &Ack) {
        let session_id = self.status.session_id;
        let fragment_index = ack.fragment_index;

        // Update packets_status using nested HashMap access
        if let Some(fragments) = self.storage.packets_status.get_mut(&session_id) {
            fragments.insert(fragment_index, PacketStatus::Sent);
        }

        // Remove from output_buffer using proper nested structure
        if let Some(fragments) = self.storage.output_buffer.get_mut(&session_id) {
            fragments.remove(&fragment_index);

            // Clean up empty session entries
            if fragments.is_empty() {
                self.storage.output_buffer.remove(&session_id);
            }
        }
    }


    fn handle_nack(&mut self, nack_packet: Packet, nack: &Nack) {
        // Handle specific NACK types
        match nack.nack_type.clone() {
            NackType::ErrorInRouting(node_id) =>  self.handle_error_in_routing(node_id, nack_packet, nack),
            NackType::DestinationIsDrone => self.handle_destination_is_drone(nack_packet, nack),
            NackType::Dropped => self.handle_packdrop(nack_packet, nack),
            NackType::UnexpectedRecipient(node_id) => self.handle_unexpected_recipient(node_id, nack_packet, nack),
        }
    }


    fn handle_error_in_routing(&mut self, node_id: NodeId, nack_packet: Packet, nack: &Nack) {
        // Clean up packet_send connection
        if self.communication_tools.packet_send.remove(&node_id).is_some() {
            warn!("Removed broken connection to node {} from packet_send", node_id);
        }

        warn!("Routing error encountered for node {}: Drone crashed or sender not found", node_id);

        let session_id = self.status.session_id;
        let fragment_index = nack.fragment_index;

        self.update_packet_status(
            session_id,
            fragment_index,
            PacketStatus::NotSent(NotSentType::RoutingError),
        );
        let opt_packet = self.storage.output_buffer
            .get_mut(&session_id)
            .and_then(|fragments| fragments.get_mut(&fragment_index))
            .cloned();

        let packet_to_send = {
            if let Some(mut packet) = opt_packet {
                let opt_destination = packet.routing_header.destination();
                if let Some(destination) = opt_destination {
                    let pack = match self.communication.routing_table.get(&destination) {
                        Some(routes) => {
                            if routes.contains_key(&packet.routing_header.hops) {
                                //do flooding because if there is still the wrong path in the routing table
                                //means that the routing table is not cleared up and therefore the
                                //flooding is not initiated
                                self.do_flooding();
                                None
                            } else if !routes.is_empty() {
                                // Limit the scope of the mutable borrow here
                                let source_routing_header = self.get_source_routing_header(destination);
                                if let Some(srh) = source_routing_header {
                                    packet.routing_header = srh; // Perform the update
                                    Some(packet.clone()) // Clone the packet
                                } else {
                                    None
                                }
                            } else {  //when the routing table doesn't contain the wrong route and is empty
                                None
                            }
                        }
                        None => None,
                    };
                    pack
                } else {
                    None
                }
            } else {
                warn!(
            "Packet not found in output buffer (Session: {}, Fragment: {})",
            session_id, fragment_index
        );
                None
            }
        };
        // Second mutable borrow for sending (after first borrow is dropped)
        if let Some(p) = packet_to_send {
            self.send(p);
        }
    }
    fn handle_destination_is_drone(&mut self, nack_packet: Packet, nack: &Nack) {
        self.update_packet_status(self.status.session_id, nack.fragment_index, PacketStatus::NotSent(NotSentType::DroneDestination));
        self.storage.output_buffer.remove(&self.status.session_id);
        //the post-part of the handling is in the send_packets_in_buffer_checking_status
    }
    fn handle_packdrop(&mut self, nack_packet: Packet, nack: &Nack) {
        self.update_packet_status(self.status.session_id, nack.fragment_index, PacketStatus::NotSent(NotSentType::Dropped));
        let packet = self.storage.output_buffer.get_mut(&self.status.session_id).unwrap().get(&nack.fragment_index).unwrap().clone();
        self.send(packet);
    }

    fn handle_unexpected_recipient(&mut self, node_id: NodeId, nack_packet: Packet, nack: &Nack) {
        info!("unexpected recipient found {}", node_id);
        let session_id = self.status.session_id;
        let fragment_index = nack.fragment_index;

        self.update_packet_status(
            self.status.session_id,
            nack.fragment_index,
            PacketStatus::NotSent(NotSentType::BeenInWrongRecipient));

        let opt_packet = self.storage.output_buffer
            .get_mut(&session_id)
            .and_then(|fragments| fragments.get_mut(&fragment_index))
            .cloned();

        let packet_to_send = {
            if let Some(mut packet) = opt_packet {
                let opt_destination = packet.routing_header.destination();
                if let Some(destination) = opt_destination {
                    let pack = match self.communication.routing_table.get(&destination) {
                        Some(routes) => {
                            if routes.contains_key(&packet.routing_header.hops) {
                                //do flooding because if there is still the wrong path in the routing table
                                //means that the routing table is not cleared up and therefore the
                                //flooding is not initiated
                                self.do_flooding();
                                None
                            } else if !routes.is_empty() {
                                // Limit the scope of the mutable borrow here
                                let source_routing_header = self.get_source_routing_header(destination);
                                if let Some(srh) = source_routing_header {
                                    packet.routing_header = srh; // Perform the update
                                    Some(packet.clone()) // Clone the packet
                                } else {
                                    None
                                }
                            } else {  //when the routing table doesn't contain the wrong route and is empty
                                None
                            }
                        }
                        None => None,
                    };
                    pack
                } else {
                    None
                }
            } else {
                warn!(
            "Packet not found in output buffer (Session: {}, Fragment: {})",
            session_id, fragment_index
        );
                None
            }
        };
        // Second mutable borrow for sending (after first borrow is dropped)
        if let Some(p) = packet_to_send {
            self.send(p);
        }
    }
}