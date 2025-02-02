use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::{ClientChen, Sending, ServerQuery};
use crate::clients::client_chen::general_client_traits::*;

impl ServerQuery for ClientChen{

    fn ask_server_type(&mut self, server_id: ServerId) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::AskType);
        }
    }

    fn ask_list_files(&mut self, server_id: ServerId) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::AskListFiles);
        }
    }

    fn ask_file(&mut self, server_id: ServerId, file_ref: String) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::AskFile(file_ref));
        }
    }

    fn ask_media(&mut self, server_id: ServerId, media_ref: String) {
        if self.get_discovered_servers_from_topology().contains(&server_id) {
            self.send_query(server_id, Query::AskMedia(media_ref));
        }
    }
}