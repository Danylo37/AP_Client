use crate::general_use::{ClientCommand, ClientId, ClientType, Response, ServerId, ServerType};
use crate::simulation_controller::SimulationController;
use crossbeam_channel::Receiver;
use std::collections::HashMap;
use std::io::{self, Write};
use wg_2024::network::NodeId;

pub struct UI<'a> {
    controller: &'a mut SimulationController,
    response_recv: Receiver<Response>,
    clients: HashMap<ClientId, Vec<ClientId>>,
    servers: HashMap<ClientId, Vec<(ServerId, ServerType, bool)>>,
}

impl<'a> UI<'a> {
    pub fn new(controller: &'a mut SimulationController, response_recv: Receiver<Response>) -> Self {
        Self {
            controller,
            response_recv,
            clients: HashMap::new(),
            servers: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            println!(
                "\nChoose an option\n\
                1. Use clients\n\
                2. Crashing a drone\n\
                0. Exit"
            );
            let user_choice = Self::ask_input_user();

            match user_choice {
                1 => self.use_clients(),
                2 => self.crash_drone(),
                0 => break,
                _ => println!("Not a valid option, choose again"),
            }
        }
    }

    fn ask_input_user() -> usize {
        loop {
            print!("> ");
            io::stdout().flush().unwrap();

            match Self::take_user_input_and_parse() {
                Some(input) => return input,
                None => {
                    println!("Invalid input. Please try again.");
                }
            }
        }
    }

    fn take_user_input_and_parse() -> Option<usize> {
        let mut user_input = String::new();
        if let Err(err) = io::stdin().read_line(&mut user_input) {
            eprintln!("Error reading input: {}", err);
            return None;
        }

        user_input.trim().parse().ok()
    }

    fn use_clients(&mut self) {
        let mut stay_inside = true;
        while stay_inside {
            let clients = self.controller.get_list_clients();

            if clients.is_empty() {
                println!("\nNo clients found");
                return;
            }

            println!("\nWhich client?");
            for (i, client) in clients.iter().enumerate(){
                println!("{}. {} client {}", i+1, client.0, client.1);
            }
            println!("0. Go back");

            let user_choice = Self::ask_input_user();

            match user_choice {
                0 => stay_inside = false,
                x if (0..=clients.len()).contains(&(x-1)) => {
                    let client_chose = clients[user_choice - 1];

                    match client_chose.0 {
                        ClientType::Chat => self.chat_client_menu(client_chose.1),
                        ClientType::Web => self.web_client_menu(client_chose.1),
                    }
                }
                _ => println!("Not a valid option, choose again")
            }
        }
    }

    fn chat_client_menu(&mut self, client_id_chose: NodeId) {
        let mut stay_inside = true;
        while stay_inside {
            println!(
                "\nChoose client function:\n\
                1. Start flooding\n\
                2. Ask type to the servers\n\
                3. Register to a server\n\
                4. List clients\n\
                5. Send message\n\
                0. Go back"
            );

            let user_choice = Self::ask_input_user();

            match user_choice {
                1 => self.start_flooding(client_id_chose),
                2 => self.ask_type(client_id_chose),
                3 => self.register_to_server(client_id_chose),
                4 => self.ask_list_clients(client_id_chose),
                5 => self.send_message_to(client_id_chose),
                0 => stay_inside = false,
                _ => println!("Not a valid option, choose again")
            }
        }
    }

    fn web_client_menu(&mut self, client_id_chose: NodeId) {
        let mut stay_inside = true;
        while stay_inside {
            println!(
                "\nChoose client function:\n\
                1. Start flooding\n\
                2. Ask type to the servers\n\
                3. Ask list files\n\
                4. Ask file\n\
                5. Ask media\n\
                0. Go back"
            );

            let user_choice = Self::ask_input_user();

            match user_choice {
                1 => self.start_flooding(client_id_chose),
                2 => self.ask_type(client_id_chose),
                3 => self.ask_list_files(client_id_chose),
                4 => self.ask_file(client_id_chose),
                5 => self.ask_media(client_id_chose),
                0 => stay_inside = false,
                _ => println!("Not a valid option, choose again")
            }
        }
    }

    fn start_flooding(&mut self, client_id_chose: NodeId) {
        self.controller.start_flooding_on_client(client_id_chose).expect("TODO: panic message");
    }

    fn ask_type(&mut self, client_id: NodeId) {
        let Some(server_id) = self.choose_server(client_id) else {
            return;
        };

        println!("Asking type to server {}", server_id);
        self.controller.ask_server_type_with_client_id(client_id, server_id).unwrap();

        match self.response_recv.recv() {
            Ok(response) => {
                match response {
                    Response::ServerType(server_type) => {
                        println!("Server type: {:?}", server_type);
                    }
                    _ => {
                        println!("Unexpected response");
                    }
                }
            }
            Err(err) => {
                eprintln!("Error receiving response: {}", err);
            }
        }
    }


    fn register_to_server(&mut self, client_id: NodeId) {
        let Some(server_id) = self.choose_server(client_id) else {
            return;
        };

        println!("Asking to register to server {}", server_id);
        self.controller
            .command_senders_clients
            .get(&client_id)
            .unwrap()
            .0
            .send(ClientCommand::RegisterToServer(server_id))
            .unwrap();

        match self.response_recv.recv() {
            Ok(response) => {
                match response {
                    Response::ClientRegistered => {
                        println!("Client registered to server {} successfully", server_id);
                    }
                    Response::Err(err) => {
                        println!("Error registering to server: {}", err);
                    }
                    _ => {
                        println!("Unexpected response");
                    }
                }
            }
            Err(err) => {
                eprintln!("Error receiving response: {}", err);
            }
        }
    }

    fn ask_list_clients(&mut self, client_id: NodeId) {
        let Some(server_id) = self.choose_server(client_id) else {
            return;
        };

        println!("Requesting clients list from server {}", server_id);
        self.controller
            .command_senders_clients
            .get(&client_id)
            .unwrap()
            .0
            .send(ClientCommand::AskListClients(server_id))
            .unwrap();

        match self.response_recv.recv() {
            Ok(response) => {
                match response {
                    Response::ListClients(mut list) => {
                        println!("Clients list {:?}", list);
                        list.retain(|&id| id != client_id);
                        self.clients.insert(client_id, list);
                    }
                    _ => {
                        println!("Unexpected response");
                    }
                }
            }
            Err(err) => {
                eprintln!("Error receiving response: {}", err);
            }
        }
    }

    fn choose_server(&mut self, client_id: NodeId) -> Option<NodeId> {
        let mut stay_inside = true;
        while stay_inside {
            self.print_servers(client_id);

            println!("\nWhich server do you want to choose?");
            let user_choice = Self::ask_input_user();

            match user_choice {
                // x if (0..=self.servers.get(&client_id).unwrap().len()).contains(&(x-1)) => {
                //     return Some(self.servers.get(&client_id).unwrap()[user_choice - 1].0);
                // }
                1 => return Some(8),
                2 => return Some(14),
                3 => return Some(17),
                0 => return None,
                _ => println!("Not a valid option, choose again")
            }
        }
        None
    }

    fn print_servers(&mut self, _client_id: ClientId) {
        println!("1. Communication server 8");
        println!("2. Media server 14");
        println!("3. Text server 17");
        println!("0. Go back");

        // if let Some(servers) = self.servers.get(&client_id) {
        //     if servers.is_empty() {
        //         println!("\nNo servers available");
        //         return;
        //     }
        //
        //     for (i, (server_id, server_type, is_registered)) in servers.iter().enumerate() {
        //         print!("{}. {} Server {}", i+1, server_id, server_type);
        //         if *is_registered {
        //             println!(" (registered)");
        //         }
        //     }
        //     println!("0. Go back");

        // } else {
        //     match self.controller.request_known_servers(client_id) {
        //         Ok(servers) => {
        //             self.servers.insert(client_id, servers);
        //             self.print_servers(client_id);
        //         }
        //         Err(err) => {
        //             eprintln!("Error requesting servers: {}", err);
        //         }
        //     };
        // }
    }

    fn send_message_to(&mut self, client_id: NodeId) {
        let mut stay_inside = true;
        while stay_inside {
            let Some(clients_ids) = self.clients.get(&client_id) else {
                println!("\nNo clients available to send message to");
                return;
            };

            if clients_ids.is_empty() {
                println!("\nNo clients available to send message to");
                return;
            }

            println!("\nWhich client do you want to send the message to?");
            for (i, client) in clients_ids.iter().enumerate(){
                println!("{}. Client {}", i+1, client);
            }
            println!("0. Go back");

            let user_choice = Self::ask_input_user();

            let message = "Message".to_string();

            match user_choice {
                0 => stay_inside = false,
                x if (0..=clients_ids.len()).contains(&(x-1)) => {
                    let client_id_chose = clients_ids[user_choice - 1];

                    self.controller
                        .command_senders_clients
                        .get(&client_id)
                        .unwrap()
                        .0
                        .send(ClientCommand::SendMessageTo(client_id_chose, message))
                        .unwrap();

                    println!("\nMessage sent to client {}", client_id_chose);

                    match self.response_recv.recv() {
                        Ok(response) => {
                            match response {
                                Response::MessageReceived(message) => {
                                    println!("Client {} received message from client {}: {}", client_id_chose, message.get_sender(), message.get_content());
                                }
                                _ => {
                                    println!("Unexpected response");
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
                _ => println!("Not a valid option, choose again")
            }
        }
    }

    fn ask_list_files(&mut self, client_id: NodeId) {
        let Some(server_id) = self.choose_server(client_id) else {
            return;
        };

        println!("Requesting files list from server {}", server_id);
        self.controller
            .command_senders_clients
            .get(&client_id)
            .unwrap()
            .0
            .send(ClientCommand::RequestListFile(server_id))
            .unwrap();

        match self.response_recv.recv() {
            Ok(response) => {
                match response {
                    Response::ListFiles(list) => {
                        println!("Files list {:?}", list);
                    }
                    _ => {
                        println!("Unexpected response");
                    }
                }
            }
            Err(err) => {
                eprintln!("Error receiving response: {}", err);
            }
        }
    }

    fn ask_file(&mut self, client_id: NodeId) {
        let Some(server_id) = self.choose_server(client_id) else {
            return;
        };

        let file = "file.txt".to_string();  // todo

        println!("Requesting file from server {}", server_id);
        self.controller
            .command_senders_clients
            .get(&client_id)
            .unwrap()
            .0
            .send(ClientCommand::RequestText(server_id, file))
            .unwrap();

        match self.response_recv.recv() {
            Ok(response) => {
                match response {
                    Response::File(file) => {
                        println!("File {:?}", file);
                    }
                    _ => {
                        println!("Unexpected response");
                    }
                }
            }
            Err(err) => {
                eprintln!("Error receiving response: {}", err);
            }
        }
    }

    fn ask_media(&mut self, client_id: NodeId) {
        let Some(server_id) = self.choose_server(client_id) else {
            return;
        };

        let media = "media.mp4".to_string();  // todo

        println!("Requesting media from server {}", server_id);
        self.controller
            .command_senders_clients
            .get(&client_id)
            .unwrap()
            .0
            .send(ClientCommand::RequestMedia(server_id, media))
            .unwrap();

        match self.response_recv.recv() {
            Ok(response) => {
                match response {
                    Response::Media(media) => {
                        println!("Media {:?}", media);
                    }
                    _ => {
                        println!("Unexpected response");
                    }
                }
            }
            Err(err) => {
                eprintln!("Error receiving response: {}", err);
            }
        }
    }

    fn crash_drone(&mut self) {
        println!("Crush drone 15");
        self.controller.request_drone_crash(15).unwrap()
    }
}