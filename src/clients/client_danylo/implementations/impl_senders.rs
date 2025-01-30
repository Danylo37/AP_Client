use log::{debug, error, info};

use wg_2024::{
    packet::Packet,
    network::SourceRoutingHeader,
};

use crate::general_use::{ClientEvent, FragmentIndex, SessionId};
use super::{Senders, ChatClientDanylo, CommandHandler};

impl Senders for ChatClientDanylo {
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
        match sender.send(packet.clone()) {
            Ok(_) => {
                // Send the 'PacketSent' event to the simulation controller
                self.send_event(ClientEvent::PacketSent(packet));
                Ok(())
            }
            Err(err) => {
                Err(format!("Failed to send packet to next hop: {}", err))
            }
        }
    }

    /// ###### Sends an acknowledgment (ACK) for a received fragment.
    /// Creates an ACK packet and sends it to the next hop.
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

    /// ###### Sends an event to the simulation controller.
    fn send_event(&self, event: ClientEvent) {
        let result = self.controller_send.send(event.clone());
        let event_name = match event {
            ClientEvent::PacketSent(_) => "PacketSent",
            ClientEvent::KnownServers(_) => "KnownServers",
            ClientEvent::ChatClientData(_, _, _) => "ChatClientData",
            _ => "Unknown",
        };

        match result {
            Ok(_) => info!("Client {}: Sent '{}' event to controller", self.id, event_name),
            Err(_) => error!("Client {}: Error sending '{}' event to controller", self.id, event_name),
        }
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

    /// ###### Resends queries that were waiting for the route to the server.
    /// Iterates over the queries to resend and sends them to the corresponding servers.
    /// If the server does not have a route, the query is not resent.
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
}