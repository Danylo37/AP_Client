use crate::clients::client_chen::{ClientChen, PacketsReceiver, PacketResponseHandler, FragmentsHandler, FloodingPacketsHandler};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
impl PacketsReceiver for ClientChen {
    fn handle_received_packet(&mut self, packet: Packet) {
        // Store in input_packet_disk
        let mut fragment_index:FragmentIndex = 0;
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

    }
}


