use std::collections::HashMap;
use std::io::{self, Write};
use wg_2024::network::NodeId;
use crate::clients::Client;
use crate::general_use::ClientCommand;
use crate::simulation_controller::SimulationController;

pub(crate) fn start_testing(mut controller: SimulationController) {
    let mut clients: HashMap<i32, NodeId> = HashMap::new();
    let mut counter = 0;

    println!("You want to wait a little Y/N?");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read input");

    println!("Which client do you want to choose?");
    for client_id in controller.command_senders_clients.keys() {
        clients.insert(counter, client_id.clone());
        counter += 1;
        println!("{}. Client with id: {} of client type {:?}", counter, client_id, controller.command_senders_clients.get(client_id).unwrap().1);
    }

    print!("Enter your choice: ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).expect("Failed to read input");

    let choice: usize = match choice.trim().parse::<usize>() {
        Ok(num) if num > 0 && num <= clients.len() => num - 1,
        _ => {
            println!("Invalid choice.");
            return;
        }
    };

    let chosen_client = clients.get(&(choice as i32)).unwrap();


    //todo()! maybe try to send to each node the flood request through the network and see which nodes doesn't work
    loop {
        println!(
            "Which operation do you want to test?\n\
             1. Test Flooding\n\
             2. Test Requesting List File\n\
             3. Test Crash Drone in a Route to test the Nack and Ack\n\
             4. Requesting a Text\n\
             5. Requesting a Media\n\
             6. Exit"
        );

        print!("Enter operation number: ");
        io::stdout().flush().unwrap();

        let mut operation = String::new();
        io::stdin().read_line(&mut operation).expect("Failed to read input");

        let operation: usize = match operation.trim().parse::<usize>() {
            Ok(num) if (1..=6).contains(&num) => num,
            _ => {
                println!("Invalid operation choice.");
                continue;
            }
        };

        match operation {
            1 => {
                if let Some((cmd_tx, _)) = controller.command_senders_clients.get(chosen_client) {
                    let sender = cmd_tx.clone();
                    sender.send(ClientCommand::StartFlooding).unwrap();
                    eprintln!("Flooding command sent to the client");
                }
            }
            2 => {
                println!("From which server?");
                let servers: Vec<&NodeId> = controller.command_senders_servers.keys().collect();

                for (idx, server) in servers.iter().enumerate() {
                    println!("{}. Text Server with id: {} of type: {:?}", idx + 1, server, controller.command_senders_clients.get(server).unwrap().1);
                }

                print!("Enter server choice: ");
                io::stdout().flush().unwrap();

                let mut choice = String::new();
                io::stdin().read_line(&mut choice).expect("Failed to read input");

                let choice: usize = match choice.trim().parse::<usize>() {
                    Ok(num) if num > 0 && num <= servers.len() => num - 1,
                    _ => {
                        println!("Invalid choice.");
                        continue;
                    }
                };

                if let Some((cmd_tx, _)) = controller.command_senders_clients.get(chosen_client) {
                    let sender = cmd_tx.clone();
                    sender.send(ClientCommand::RequestListFile(*servers[choice]))
                        .unwrap();
                }
            }
            3 => {
                println!("You want to test on the route of which text server?");
                let text_servers: Vec<&NodeId> = vec![];

                for (idx, server) in text_servers.iter().enumerate() {
                    println!("{}. Text Server with id: {}", idx + 1, server);
                }

                print!("Enter server choice: ");
                io::stdout().flush().unwrap();

                let mut choice = String::new();
                io::stdin().read_line(&mut choice).expect("Failed to read input");

                let choice: usize = match choice.trim().parse::<usize>() {
                    Ok(num) if num > 0 && num <= text_servers.len() => num - 1,
                    _ => {
                        println!("Invalid choice.");
                        continue;
                    }
                };

                let chosen_server = *text_servers[choice];

                if let Some((cmd_tx, _)) = controller.command_senders_clients.get(chosen_client) {
                    let sender = cmd_tx.clone();
                    sender.send(ClientCommand::RequestRoutes(chosen_server)).unwrap();
                }

                println!("Now, which Drone do you want to crash?");
                print!("Enter drone ID: ");
                io::stdout().flush().unwrap();

                let mut drone_choice = String::new();
                io::stdin().read_line(&mut drone_choice).expect("Failed to read input");

                let drone_choice: i32 = match drone_choice.trim().parse::<i32>() {
                    Ok(num) => num,
                    Err(_) => {
                        println!("Invalid drone ID.");
                        continue;
                    }
                };

                // Mutable borrow happens here, but after immutable borrow ends
                controller.request_drone_crash(drone_choice as NodeId).expect("Drone crash failed");

                if let Some((cmd_tx, _)) = controller.command_senders_clients.get(chosen_client) {
                    let sender = cmd_tx.clone();
                    sender.send(ClientCommand::AskTypeTo(chosen_server)).unwrap();
                }
            }
            4 | 5 => {
                println!("Feature not implemented yet.");
            }
            6 => {
                println!("Exiting testing mode.");
                break;
            }
            _ => {
                println!("Invalid choice.");
            }
        }
    }
}
