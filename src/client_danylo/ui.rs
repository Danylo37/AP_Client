use std::{io, io::Write};
use wg_2024::network::NodeId;
use super::client_danylo::ChatClientDanylo;

pub fn start_ui(mut client: ChatClientDanylo) {
    loop {
        println!(
            "1. Send request to server\n\
             2. Discover the network\n\
             3. Exit"
        );
        print!("> ");
        io::stdout().flush().unwrap();

        let user_choice = ask_input_user();

        match user_choice {
            1 => { send_query_menu(&mut client) }
            2 => { client.discovery() }
            3 => break,
            _ => println!("Not a valid option, choose again"),
        }
        println!();
    }
}

fn ask_input_user() -> i32 {
    loop {
        let user_input = take_user_input_and_parse();
        if user_input != -1 {
            return user_input;
        }
    }
}

fn take_user_input_and_parse() -> i32 {
    let mut user_input = String::new();
    io::stdin()
        .read_line(&mut user_input)
        .expect("Error in reading your choice");

    user_input.trim().parse().unwrap_or_else(|e| {
        println!("Error in parse: {} \n Try again \n", e);
        print!("> ");
        io::stdout().flush().unwrap();
        -1
    })
}

fn send_query_menu(client: &mut ChatClientDanylo) {
    let servers: Vec<NodeId> = client.servers.keys().cloned().collect();

    if servers.is_empty() {
        println!("No servers found.");
        return;
    }

    let server_list = get_server_list(&servers);
    let last_option_number = servers.len() + 1;

    loop {
        println!("\nChoose server:\n\
                {}\
                {}. Return back"
                 , server_list, last_option_number);
        print!("> ");
        io::stdout().flush().unwrap();

        let user_choice = ask_input_user();

        if user_choice > 0 && user_choice <= servers.len() as i32 {
            server_menu(servers[(user_choice-1) as usize]);
        } else if user_choice == last_option_number as i32 {
            return;
        } else {
            println!("Not a valid option, please choose again.");
        }
    }
}

fn get_server_list(server_ids: &Vec<NodeId>) -> String {
    let mut list = "".to_string();
    let mut counter = 1;
    for id in server_ids {
        list.push_str(&format!("{}. Server with ID {}\n", counter, id));
        counter += 1;
    };

    list
}

fn server_menu(server_id: NodeId) {
    loop {
        println!("\nChoose an action:\n\
                  1. Request server type\n\
                  2. Request user's list\n\
                  3. Send message\n\
                  4. Return back");

        print!("> ");
        io::stdout().flush().unwrap();

        let user_choice = ask_input_user();

        match user_choice {
            1 => { request_server_type(server_id) }
            2 => { request_users_list(server_id) }
            3 => { send_message_menu(server_id) }
            4 => return,
            _ => println!("Not a valid option, please choose again."),
        }
    }
}

fn request_server_type(_server_id: NodeId) {
    // Add logic for requesting server type
    println!("Requesting server type");
}

fn request_users_list(_server_id: NodeId) {
    // Add logic for requesting user's list
    println!("Requesting user's list");
}

fn send_message_menu(server_id: NodeId) {
    let users = get_users_list(server_id);

    if users.is_empty() {
        println!("No users found. Please request the user list from the server.");
        return;
    }

    println!("\nChoose a user to send a message:");
    for (index, user) in users.iter().enumerate() {
        println!("{}. {}", index + 1, user);
    }
    println!("{}: Return back", users.len() + 1);

    print!("> ");
    io::stdout().flush().unwrap();

    let user_choice = ask_input_user();

    if user_choice > 0 && user_choice <= users.len() as i32 {
        send_message_to_user(users[(user_choice - 1) as usize].clone());
    } else if user_choice == users.len() as i32 + 1 {
        return;
    } else {
        println!("Not a valid option, please choose again.");
    }
}

fn get_users_list(_server_id: NodeId) -> Vec<String> {
    // Add logic to retrieve users from the server
    vec!["User1".to_string(), "User2".to_string()] // Placeholder
}

fn send_message_to_user(user: String) {
    println!("Sending message to {}", user);
    // Add logic for sending a message to the user
}
