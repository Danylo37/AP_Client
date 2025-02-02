use serde::de::DeserializeOwned;
use crate::clients::client_chen::{ClientChen, FragmentsHandler, PacketCreator, PacketsReceiver, Sending, SpecificInfo};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
use crate::clients::client_chen::web_browser_client_traits::WebBrowserClientTrait;

impl FragmentsHandler for ClientChen {
    fn handle_fragment(&mut self, msg_packet: Packet, fragment: &Fragment) {
        self.storage.fragment_assembling_buffer
            .entry(msg_packet.session_id)
            .or_insert_with(HashMap::new)
            .insert(fragment.fragment_index, msg_packet.clone());
        // Send an ACK instantly
        if let Some(destination) = msg_packet.routing_header.destination() {
            if destination != self.metadata.node_id{
                let nack = self.create_nack_packet_from_receiving_packet(msg_packet.clone(), NackType::UnexpectedRecipient(self.metadata.node_id));
                self.send(nack);
                return;
            } else{
                let ack_packet = self.create_ack_packet_from_receiving_packet(msg_packet.clone());
                self.send(ack_packet);
            }
        } else{
            panic!("The fragment has no destination, so the fragment is sent casually");
        }
    }

    fn get_total_n_fragments(&self, session_id: SessionId) -> Option<u64> {
        self.storage
            .fragment_assembling_buffer
            .get(&session_id)?
            .values()
            .next()  // Get first packet in the session
            .and_then(|packet| {
                if let PacketType::MsgFragment(fragment) = &packet.pack_type {
                    Some(fragment.total_n_fragments)
                } else {
                    None
                }
            })
    }

    fn get_fragments_quantity_for_session(&self, session_id: SessionId) -> Option<u64> {
        Some(
            self.storage
                .fragment_assembling_buffer
                .get(&session_id)?
                .len() as u64
        )
    }

    fn handle_fragments_in_buffer_with_checking_status(&mut self) {
        // Collect session IDs to avoid borrowing issues during iteration
        let session_ids: Vec<_> = self
            .storage
            .fragment_assembling_buffer
            .keys()
            .cloned()
            .collect();

        // Iterate over each session and process fragments
        for session_id in session_ids {
            if let Some(total_n_fragments) = self.get_total_n_fragments(session_id) {
                if let Some(fragments_quantity) = self.get_fragments_quantity_for_session(session_id) {
                    if fragments_quantity == total_n_fragments {
                        if let Some(first_packet) = self
                            .storage
                            .fragment_assembling_buffer
                            .get(&session_id)
                            .and_then(|fragments| fragments.values().next())
                        {
                            let initiator_id = first_packet.routing_header.destination();

                            // Reassemble fragments and process the message
                            if let Ok(message) = self.reassemble_fragments_in_buffer(session_id) {
                                if let Some(id) = initiator_id {
                                    self.process_message(id, message);
                                    self.storage.fragment_assembling_buffer.remove(&session_id);
                                } else {
                                    warn!("Initiator ID not found for session: {:?}", session_id);
                                }
                            } else {
                                warn!(
                                    "Failed to reassemble fragments for session: {:?}",
                                    session_id
                                );
                            }
                        } else {
                            warn!("No fragments found for session: {:?}", session_id);
                        }
                    }
                }
            }
        }
    }


    fn process_message(&mut self, initiator_id: NodeId, message: Response) {
        match message {
            Response::ServerType(server_type) => {
                self.update_topology_entry_for_server(initiator_id, server_type);
                println!("The type of the server is {:?}", server_type);
            },
            Response::ListFiles(list_file)  => {
                // Placeholder for file/media handling
                self.handle_list_file(list_file);
            },
            Response::File(text) => {
                self.handle_text_file(text);
            },
            Response::Media(media) =>{
                self.handle_media(media);
            },
            Response::Err(error) => {
                warn!("Error received: {:?}", error);
            },
            _ => {}
        }
    }

    fn reassemble_fragments<T: Serialize + DeserializeOwned>(&mut self, fragments: Vec<Packet>) -> Result<T, String> {
        let mut raw_data = Vec::new();

        for packet in fragments {
            match &packet.pack_type {
                PacketType::MsgFragment(fragment) => {
                    // Push only the valid portion of `data`
                    raw_data.extend_from_slice(&fragment.data[..fragment.length as usize]);
                }
                _ => return Err("Non-fragment packet type".to_string()),
            }
        }

        // Convert bytes to String once at the end
        let serialized_msg = String::from_utf8(raw_data)
            .map_err(|e| format!("Invalid UTF-8 sequence: {}", e))?;

        eprintln!("Reassembled JSON: {:?}", serialized_msg); // Debugging

        // Deserialize the complete message
        serde_json::from_str(&serialized_msg)
            .map_err(|e| format!("Deserialization failed: {}", e))
    }
    fn reassemble_fragments_in_buffer<T: Serialize + DeserializeOwned>(&mut self, session_id: SessionId) -> Result<T, String> {
        // Get fragments once to avoid multiple lookups
        let fragments = self.storage
            .fragment_assembling_buffer
            .get(&session_id)
            .ok_or_else(|| format!("Session {} not found in buffer", session_id))?;

        // Collect and sort fragment indices
        let mut keys: Vec<FragmentIndex> = fragments.keys().cloned().collect();
        keys.sort();

        // Pre-allocate buffer for better performance
        let mut raw_data = Vec::new();

        for key in keys {
            let packet = fragments.get(&key)
                .ok_or_else(|| format!("Fragment {} missing in session {}", key, session_id))?;

            match &packet.pack_type {
                PacketType::MsgFragment(fragment) => {
                    raw_data.extend_from_slice(&fragment.data[..fragment.length as usize]);
                }
                _ => return Err(format!("Non-fragment packet type in session {} at index {}", session_id, key)),
            }
        }

        // Convert bytes to String once at the end
        let serialized_msg = String::from_utf8(raw_data)
            .map_err(|e| format!("Invalid UTF-8 sequence: {}", e))?;

        // Deserialize the complete message
        serde_json::from_str(&serialized_msg)
            .map_err(|e| format!("Deserialization failed: {}", e))
    }
}
