use crate::clients::client_chen::{ClientChen, PacketCreator};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
impl PacketCreator for ClientChen{
    fn divide_string_into_slices(&mut self, string: String, max_slice_length: usize) -> Vec<String> {
        let mut slices = Vec::new();
        let mut start = 0;

        while start < string.len() {
            // Compare the (start + max_slice_length) with the end of the string to give us a provvisory end
            let end = std::cmp::min(start + max_slice_length, string.len());
            println!("end: {}", end);
            // Validate the end
            let valid_end = string[..end].char_indices().last().map(|(idx, _)| idx + 1).unwrap_or(end);

            slices.push(string[start..valid_end].to_string());
            start = valid_end;
        }
        println!("finish slicing: {:?}", slices);
        slices
    }

    fn msg_to_fragments<T: Serialize>(&mut self, msg: T, destination_id: NodeId) -> Option<Vec<Packet>> {
        let serialized_msg = serde_json::to_string(&msg).unwrap();
        let mut fragments = Vec::new(); //fragments are of type Packet
        let msg_slices = self.divide_string_into_slices(serialized_msg, FRAGMENT_DSIZE);
        let number_of_fragments = msg_slices.len();

        if let Some(source_routing_header) = self.get_source_routing_header(destination_id){
            self.status.session_id += 1;
            //the i is counted from 0 so it's perfect suited in our case
            for (i, slice) in msg_slices.into_iter().enumerate() {
                //Convert each slice of the message into the same format of the field data of the struct Fragment.
                let slice_bytes = slice.as_bytes();
                let fragment_data = {
                    let mut buffer = [0u8; FRAGMENT_DSIZE]; // Create a buffer with the exact required size
                    let slice_length = std::cmp::min(slice_bytes.len(), FRAGMENT_DSIZE); // Ensure no overflow
                    buffer[..slice_length].copy_from_slice(&slice_bytes[..slice_length]);
                    buffer
                };

                let fragment = Fragment {
                    fragment_index: i as u64,
                    total_n_fragments: number_of_fragments as u64,
                    length: slice.len() as u8, //Note u8 is 256 but actually "length <= FRAGMENT_DSIZE = 128"
                    data: fragment_data,       //Fragment data
                };

                let routing_header = source_routing_header.clone();
                let packet = Packet::new_fragment(routing_header, self.status.session_id, fragment);
                fragments.push(packet);
            }
            Some(fragments)
        }else{
            None
        }
    }

    fn msg_to_fragments_by_routing_header<T: Serialize>(&mut self, msg: T, source_routing_header: SourceRoutingHeader) -> Option<Vec<Packet>> {
        if source_routing_header.is_empty(){
            return None;
        }
        let serialized_msg = serde_json::to_string(&msg).unwrap();
        let mut fragments = Vec::new(); //fragments are of type Packet
        let msg_slices = self.divide_string_into_slices(serialized_msg, FRAGMENT_DSIZE);
        let number_of_fragments = msg_slices.len();
        for (i, slice) in msg_slices.into_iter().enumerate() {
            //Convert each slice of the message into the same format of the field data of the struct Fragment.
            let slice_bytes = slice.as_bytes();
            let fragment_data = {
                let mut buffer = [0u8; FRAGMENT_DSIZE]; // Create a buffer with the exact required size
                let slice_length = std::cmp::min(slice_bytes.len(), FRAGMENT_DSIZE); // Ensure no overflow
                buffer[..slice_length].copy_from_slice(&slice_bytes[..slice_length]);
                buffer
            };
            let fragment = Fragment {
                fragment_index: i as u64,
                total_n_fragments: number_of_fragments as u64,
                length: slice.len() as u8, //Note u8 is 256 but actually "length <= FRAGMENT_DSIZE = 128"
                data: fragment_data,       //Fragment data
            };
            let srh = source_routing_header.clone();
            let packet = Packet::new_fragment(srh, self.status.session_id, fragment);
            fragments.push(packet);
        }
        Some(fragments)
    }

    fn create_ack_packet_from_receiving_packet(&mut self, packet: Packet) -> Packet{
        self.status.session_id +=1 ;
        let routing_header = SourceRoutingHeader{
            hop_index : 1,
            hops: packet.routing_header.hops.iter().rev().copied().collect(),   //when you can, use Copy trait instead of Clone trait, it's more efficient.
        };   //nope we need to use the same of which is arrived.
        let ack_packet = Packet::new_ack(routing_header,
                                         self.status.session_id,
                                         match packet.clone().pack_type{
                                             PacketType::MsgFragment(fragment)=> fragment.fragment_index,
                                             _=> 0,
                                        });

        ack_packet
    }
    fn create_nack_packet_from_receiving_packet(&mut self, packet: Packet, nack_type: NackType) -> Packet{
        self.status.session_id += 1;
        let hops = packet.routing_header.hops.iter().rev().copied().collect();
        let routing_header= SourceRoutingHeader::initialize(hops);
        let nack = Nack{
            fragment_index: match packet.clone().pack_type{
                                  PacketType::MsgFragment(fragment)=> fragment.fragment_index,
                                  _=> 0 },
            nack_type,
        };
        let nack_packet = Packet::new_nack(routing_header, self.status.session_id, nack);
        nack_packet
    }
    fn get_hops_from_path_trace(&mut self, path_trace: Vec<(NodeId, NodeType)>) -> Vec<NodeId> {
        // Transform the best path into a vector of NodeId and initialize hop_index to 1
        let hops = path_trace.iter().map(|&(node_id, _)| node_id).collect();
        hops
    }

    /// find source routing header by searching the hops from the routing table
    fn get_source_routing_header(&mut self, destination_id: NodeId) -> Option<SourceRoutingHeader> {
        // Get the routes for the destination ID
        if let Some(routes) = self.communication.routing_table.get(&destination_id) {
            return Some(SourceRoutingHeader::initialize(routes.clone()));
        }
        // Return None if no valid path is found
        None
    }


}