use crate::clients::client_chen::{ClientChen, CommandHandler, SpecificInfo};
use crate::clients::client_chen::prelude::*;
use crate::clients::client_chen::general_client_traits::*;
use crate::general_use::{ServerType};

impl CommandHandler for ClientChen{
    fn handle_controller_command(&mut self, command: ClientCommand) {
        match command {
            ClientCommand::AddSender(target_node_id, sender) => {
                self.communication_tools.packet_send.insert(target_node_id, sender);
            }
            ClientCommand::RemoveSender(target_node_id) => {
                self.communication_tools.packet_send.remove(&target_node_id);
            }

            ClientCommand::StartFlooding => {
                self.do_flooding();
            }
            ClientCommand::GetKnownServers => {
                let servers: Vec<(ServerId, ServerType, bool)> = self
                    .get_discovered_servers_from_topology()
                    .iter()
                    .map(|server_id| {
                        self.network_info.topology.get(server_id).map_or(
                            // Default to undefined server info if not found
                            (*server_id, ServerType::Undefined, false),
                            |server| {
                                if let SpecificInfo::ServerInfo(server_info) = &server.specific_info {
                                    let server_type = server_info.server_type;
                                    (*server_id, server_type, false)
                                } else {
                                    (*server_id, ServerType::Undefined, false)
                                }
                            },
                        )
                    })
                    .collect();
                self.send_events(ClientEvent::KnownServers(servers));
            }

            ClientCommand::AskTypeTo(server_id) => {
                self.send_query(server_id, Query::AskType);
            }
            ClientCommand::RequestListFile(server_id) => {
                self.send_query(server_id, Query::AskListFiles);
            }
            ClientCommand::RequestText(server_id, file) => {
                self.send_query(server_id, Query::AskFile(file));
            }
            ClientCommand::RequestMedia(server_id, media_ref) => {
                self.send_query(server_id, Query::AskMedia(media_ref));
            }
            //testing command
            ClientCommand::RequestRoutes(destination_id) => {
                if let Some(routes) = self.communication.routing_table.get(&destination_id) {
                    eprintln!("The routes from {} to {} are: \n\
                     {:?}", self.metadata.node_id, destination_id, routes);
                }
            }
            _=>{}
        }
    }
}