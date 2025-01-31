use crate::clients::client_chen::{ClientChen, PacketCreator, Router, Sending};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
use crate::general_use::NotSentType::ToBeSent;

impl Sending for ClientChen {
    fn send_packets_in_buffer_with_checking_status(&mut self) {
        //looping in the sessions, but now agreed to have only one session in the buffer.
        //so the first for is pretty useless, but it doesn't hurt the program
        let sessions: Vec<SessionId> = self.storage.output_buffer.keys().cloned().collect();
        for session_id in sessions {
            if let Some(fragments) = self.storage.output_buffer.get(&session_id) {
                let fragment_indices: Vec<_> = fragments.keys().cloned().collect();

                //now we are looping in the fragment indexes to see which packets are not sent
                for fragment_index in fragment_indices {
                    if let Some(packet) = self.storage.output_buffer
                        .get(&session_id)
                        .and_then(|fragments| fragments.get(&fragment_index))
                    {
                        if let Some(status) = self.storage.packets_status
                            .get(&session_id)
                            .and_then(|fragments| fragments.get(&fragment_index))
                        {
                            //we will manage these packets based on their sending status
                            self.packets_status_sending_actions(packet.clone(), status.clone());
                        } else {
                            warn!("Missing status for session {} fragment {}", session_id, fragment_index);
                        }
                    } else {
                        warn!("Missing packet in output_packet_disk for session {} fragment {}", session_id, fragment_index);
                    }
                }
            }
        }
    }

    fn send(&mut self, packet: Packet) {
        if let Some(next_hop) = packet.routing_header.next_hop() {
            self.send_packet_to_connected_node(next_hop, packet);
        } else {
            panic!("No next hop available for packet: {:?}", packet);
        }
    }

    fn send_events(&mut self, client_event: ClientEvent) {
        self.communication_tools.controller_send.send(client_event)
            .unwrap_or_else(|e| error!("Failed to send client event: {}", e));
    }

    fn send_query(&mut self, server_id: ServerId, query: Query) {
        if let Some(query_packets) = self.msg_to_fragments(query, server_id) {
            for query_packet in query_packets {
                self.send(query_packet);
            }
        } else {
            warn!("Failed to fragment query");
        }
    }

    fn send_packet_to_connected_node(&mut self, target_node_id: NodeId, mut packet: Packet) {
        // Update routing metrics
        //eprintln!("We are sending the flood response to {}", target_node_id);
        if let Some(destination) = packet.routing_header.destination() {
            if let Some(routes) = self.communication.routing_table.get_mut(&destination) {
                routes.entry(packet.clone().routing_header.hops)
                    .and_modify(|using_times| *using_times += 1);
            }
        }

        // Store packet with proper nested structure
        let (session_id, fragment_index) = match &packet.pack_type {
            PacketType::MsgFragment(fragment) => (packet.session_id, fragment.fragment_index),
            _ => (packet.session_id, 0),
        };

        self.storage.output_buffer
            .entry(session_id)
            .or_default()
            .insert(fragment_index, packet.clone());

        self.storage.output_packet_disk
            .entry(session_id)
            .or_default()
            .insert(fragment_index, packet.clone());

        packet.routing_header.increase_hop_index();
        // Attempt to send packet
        match self.communication_tools.packet_send.get_mut(&target_node_id) {
            Some(sender) if self.communication.connected_nodes_ids.contains(&target_node_id) => {
                match sender.send(packet.clone()) {
                    Ok(_) => {
                        info!("Successfully sent packet to {}", target_node_id);
                        self.update_packet_status(session_id, fragment_index, PacketStatus::InProgress);
                    },
                    Err(e) => {
                        error!("Failed to send to {}: {}", target_node_id, e);
                        self.update_packet_status(session_id, fragment_index, PacketStatus::NotSent(ToBeSent));
                    }
                }
            }
            _ => {
                warn!("No valid connection to {}", target_node_id);
                self.update_packet_status(session_id, fragment_index, PacketStatus::NotSent(ToBeSent));
            }
        }
    }

    fn packets_status_sending_actions(&mut self, packet: Packet, packet_status: PacketStatus) {
        let destination = packet.routing_header.destination();

        match packet_status {
            PacketStatus::NotSent(not_sent_type) => {
                self.handle_not_sent_packet(packet, not_sent_type, destination.unwrap());
            }
            PacketStatus::Sent => {
                self.handle_sent_packet(packet);
            }
            _ => {} // No action needed
        }
    }

    fn handle_sent_packet(&mut self, packet: Packet) {
        let (session_id, fragment_index) = match &packet.pack_type {
            PacketType::MsgFragment(fragment) => (packet.session_id, fragment.fragment_index),
            _ => (packet.session_id, 0),
        };

        // This is already done in the ack handling, but it's ok to retain it,
        if let Some(fragments) = self.storage.output_buffer.get_mut(&session_id) {
            fragments.remove(&fragment_index);
            if fragments.is_empty() {
                self.storage.output_buffer.remove(&session_id);
            }
        }
    }

    fn handle_not_sent_packet(&mut self, mut packet: Packet, not_sent_type: NotSentType, destination: NodeId) {
        let routes = self.communication.routing_table.get(&destination);
        match not_sent_type {
            //through
            NotSentType::RoutingError | NotSentType::ToBeSent | NotSentType::BeenInWrongRecipient => {
                if routes.map_or(false, |r| !r.is_empty()) {
                    let srh = self.get_source_routing_header(destination);
                    packet.routing_header = srh.unwrap();
                    self.send(packet);
                } else {
                    warn!("No valid routes to {}", destination);
                }
            }
            NotSentType::Dropped =>{
                //resend the packet
                self.send(packet);
            }
            NotSentType::DroneDestination => {
                let (session_id, fragment_index) = match &packet.pack_type {
                    PacketType::MsgFragment(fragment) => (packet.session_id, fragment.fragment_index),
                    _ => (packet.session_id, 0),
                };
                self.storage.output_buffer
                    .entry(session_id)
                    .and_modify(|fragments| { fragments.remove(&fragment_index); });
            }
        }
    }

    fn update_packet_status(&mut self, session_id: SessionId, fragment_index: FragmentIndex, status: PacketStatus) {
        self.storage.packets_status
            .entry(session_id)
            .or_default()
            .insert(fragment_index, status);
    }
}