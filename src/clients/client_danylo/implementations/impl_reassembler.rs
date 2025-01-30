use log::{debug, error};
use crate::general_use::{SessionId, Response};
use super::{Reassembler, ChatClientDanylo};

impl Reassembler for ChatClientDanylo {
    /// ###### Reassembles the fragments for a given session into a complete message.
    /// Returns the reassembled message or an error if reassembly fails.
    fn reassemble(&mut self, session_id: SessionId) -> Option<Response> {
        debug!("Client {}: Reassembling message for session {}", self.id, session_id);

        // Retrieve the fragments for the given session.
        let fragments = match self.fragments_to_reassemble.get_mut(&session_id) {
            Some(fragments) => fragments,
            None => {
                error!("Client {}: No fragments found for session {}", self.id, session_id);
                return None;
            },
        };

        // Ensure all fragments belong to the same message by checking the total number of fragments.
        let total_n_fragments = match fragments.first() {
            Some(first) => first.total_n_fragments,
            None => {
                error!("Client {}: Fragment list is empty for session {}", self.id, session_id);
                return None;
            },
        };

        // Check if the number of fragments matches the expected total.
        if fragments.len() as u64 != total_n_fragments {
            error!(
                "Client {}: Incorrect number of fragments for session {}: expected {}, got {}",
                self.id,
                session_id,
                total_n_fragments,
                fragments.len()
            );
            return None;
        }

        // Collect data from all fragments.
        let mut result = Vec::new();
        for fragment in fragments {
            result.extend_from_slice(&fragment.data[..fragment.length as usize]);
        }

        // Convert the collected data into a string.
        let reassembled_string = match String::from_utf8(result) {
            Ok(string) => string,
            Err(err) => {
                error!(
                    "Client {}: Failed to convert data to string for session {}: {}",
                    self.id, session_id, err
                );
                return None;
            },
        };

        // Attempt to deserialize the string into an object.
        match serde_json::from_str(&reassembled_string) {
            Ok(deserialized) => Some(deserialized),
            Err(err) => {
                error!(
                    "Client {}: Failed to deserialize JSON for session {}: {}",
                    self.id, session_id, err
                );
                None
            },
        }
    }
}