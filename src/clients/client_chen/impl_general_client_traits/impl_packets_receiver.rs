use wg_2024::packet::NackType::UnexpectedRecipient;
use crate::clients::client_chen::{ClientChen, PacketsReceiver, PacketResponseHandler, FragmentsHandler, FloodingPacketsHandler};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
impl PacketsReceiver for ClientChen {
    fn handle_received_packet(&mut self, packet: Packet) {
        // Process packet reception metrics
        self.decreasing_using_times_when_receiving_packet(&packet);


        // Store in input_packet_disk
        let mut fragment_index:FragmentIndex = 0;
        let session_id = packet.session_id;

        let packet_clone = packet.clone();
        // Handle packet type without unnecessary cloning
        match packet.pack_type.clone() {
            PacketType::Nack(nack) => self.handle_nack(packet_clone, &nack),
            PacketType::Ack(ack) => self.handle_ack(packet_clone, &ack),
            PacketType::MsgFragment(fragment) => {
                self.handle_fragment(packet_clone, &fragment);
                fragment_index = fragment.fragment_index;
            },
            PacketType::FloodRequest(mut flood_request) => self.handle_flood_request(packet_clone, &mut flood_request),
            PacketType::FloodResponse(flood_response) => self.handle_flood_response(packet_clone, &flood_response),
        }

        self.storage.input_packet_disk
            .entry(session_id)
            .or_insert_with(HashMap::new)
            .insert(fragment_index, packet);

    }
    fn decreasing_using_times_when_receiving_packet(&mut self, packet: &Packet) {
        // Reverse the hops to get the correct order for path trace
        let hops: Vec<_> = packet.routing_header.hops.iter().rev().cloned().collect();

        // Ensure hops are not empty
        if hops.is_empty() {
            return; // Exit early if there are no hops
        }

        // Get the destination ID from the last hop
        let destination_id = hops.last().copied().unwrap();

        // Decrease `using_times` by 1 for the corresponding route
        if let Some(routes) = self.communication.routing_table.get_mut(&destination_id) {
            if let Some(using_times) = routes.get_mut(&hops) {
                if *using_times > 0 { // Prevent underflow
                    *using_times -= 1;
                }
            }
        }
    }
}


