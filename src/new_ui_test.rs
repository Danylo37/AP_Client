use crate::general_use::{ClientCommand, Response};
use crate::simulation_controller::SimulationController;
use crossbeam_channel::Receiver;
use std::collections::HashMap;
use std::io::{self, Write};
use wg_2024::network::NodeId;

pub struct UI<'a> {
    controller: &'a mut SimulationController,
    response_recv: Receiver<Response>,
    clients: HashMap<NodeId, Vec<NodeId>>,
}

impl<'a> UI<'a> {
    pub fn new(controller: &'a mut SimulationController, response_recv: Receiver<Response>) -> Self {
        Self {
            controller,
            response_recv,
            clients: HashMap::new(),
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
        println!("\nWhich client?");

        //Print clients, controller.get_list_clients() function that returns a vec<NodeId>
        let clients_ids = self.controller.get_list_clients();
        for (i, client) in clients_ids.iter().enumerate(){
            println!(
                "{}. {} client {}", i+1, client.0, client.1
            );
        }

        let user_choice = Self::ask_input_user();
        let client_id_chose = clients_ids[user_choice - 1].1;
        //We should do a check if the id user chose exists!!!

        self.choose_action_client(client_id_chose);
    }

    fn crash_drone(&mut self) {
        println!("Crush drone 15");
        self.controller.request_drone_crash(15).unwrap()
    }

    fn choose_action_client(&mut self, client_id_chose: NodeId) {
        //Variable that allows to go back
        let mut stay_inside = true;
        while stay_inside {
            //Choosing client function
            println!(
                "\nChoose client function?\n\
                1. Start flooding\n\
                2. Ask server something\n\
                0. Go back"
            );

            let user_choice = Self::ask_input_user();

            match user_choice {
                1 => self.start_flooding(client_id_chose),
                2 => self.ask_server_action(client_id_chose),
                0 => stay_inside = false,
                _ => println!("Not a valid option, choose again")
            }
        }
    }

    fn start_flooding(&mut self, client_id_chose: NodeId) {
        self.controller.start_flooding_on_client(client_id_chose).expect("TODO: panic message");
    }

    fn ask_server_action(&mut self, client_id_chose: NodeId) {
        //Variable that allows to go back
        let mut stay_inside = true;
        while stay_inside {

            println!(
                "\n What is your query?\n\
                1. Ask type to the servers\n\
                2. Register to a server\n\
                3. List clients\n\
                4. Send message to\n\
                0. Go back"
            );
            let user_choice = Self::ask_input_user();

            match user_choice {
                1 => self.ask_type(client_id_chose, 8),
                2 => self.register_to_server(client_id_chose, 8),
                3 => self.ask_list_clients(client_id_chose, 8),
                4 => self.send_message_to(client_id_chose),
                0 => stay_inside = false,
                _ => println!("Not a valid option, choose again")
            }
        }
    }

    fn ask_type(&mut self, client_id: NodeId, server_id: NodeId) {
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


    fn register_to_server(&mut self, client_id: NodeId, server_id: NodeId) {
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

    fn ask_list_clients(&mut self, client_id: NodeId, server_id: NodeId) {
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

    fn send_message_to(&mut self, client_id: NodeId) {
        let mut stay_inside = true;
        while stay_inside {
            let Some(clients_ids) = self.clients.get(&client_id) else {
                println!("No clients available to send message to");
                return;
            };

            if clients_ids.is_empty() {
                println!("No clients available to send message to");
                return;
            }

            println!("Which client do you want to send the message to?");
            for (i, client) in clients_ids.iter().enumerate(){
                println!("{}. Client {}", i+1, client);
            }
            println!("0. Go back");

            let user_choice = Self::ask_input_user();
            let client_id_chose = clients_ids[user_choice - 1];

            let message = "Message".to_string();

            match user_choice {
                x if (0..=clients_ids.len()).contains(&(x-1)) => {
                    self.controller
                        .command_senders_clients
                        .get(&client_id)
                        .unwrap()
                        .0
                        .send(ClientCommand::SendMessageTo(client_id_chose, message))
                        .unwrap();
                    stay_inside = false;
                    println!("Message sent to client {}", client_id_chose);

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
                0 => stay_inside = false,
                _ => println!("Not a valid option, choose again")
            }
        }
    }
}