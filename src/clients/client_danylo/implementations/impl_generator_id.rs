use crate::general_use::{FloodId, SessionId};
use super::{GeneratorId, ChatClientDanylo};

impl GeneratorId for ChatClientDanylo {
    /// ###### Generates a new session ID.
    fn generate_session_id(&mut self) -> SessionId {
        self.session_id_counter += 1;
        let next_session_id: SessionId = self.session_id_counter;
        self.parse_id(next_session_id)
    }

    /// ###### Generates a new flood ID.
    fn generate_flood_id(&mut self) -> FloodId {
        self.flood_id_counter += 1;
        let next_flood_id: FloodId = self.flood_id_counter;
        self.parse_id(next_flood_id)
    }

    /// ###### Parses the ID by concatenating the client ID and the provided ID.
    fn parse_id(&self, id: u64) -> u64 {
        format!("{}{}", self.id, id)
            .parse()
            .unwrap()
    }
}