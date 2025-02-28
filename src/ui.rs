use std::io;
use wg_2024::network::NodeId;
use crate::simulation_controller::SimulationController;

pub fn start_ui(mut controller: SimulationController) {
    loop {

        //Choosing base options
        println!(
            "Choose an option
1. Use clients
2. Crashing a drone
3. Nothing"
        );
        let user_choice = ask_input_user();

        match user_choice {
            1 => { use_clients(&mut controller) }
            2 => { crash_drone(&mut controller) },
            3 => break, //we break from the loop, thus we exit from the interaction.
            _ => println!("Not a valid option, choose again"),
        }
    }
}

fn crash_drone(controller: &mut SimulationController) {
    println!("Enter the ID of the drone to crash:");

    let mut input = String::new();

    io::stdin().read_line(&mut input).expect("Failed to read input");


    let drone_id: NodeId = match input.trim().parse() {
        Ok(id) => id,
        Err(_) => {
            println!("Invalid input. Please enter a valid drone ID.");
            return; // Or handle the error differently (e.g., loop until valid input)
        }
    };

    match controller.request_drone_crash(drone_id) {
        Ok(()) => println!("Crash command sent to drone {}", drone_id),
        Err(e) => println!("Error: {}", e),  // Display the specific error returned by request_drone_crash
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
    io::stdin() //get the input from the keyboard, in this case I expect a number
        .read_line(&mut user_input)
        .expect("Error in reading your choice");
    user_input.trim().parse().unwrap_or_else(|e| {
        println!("Error in parse: {} \n Try again \n", e);
        -1
    })
}

//Maybe useful later
fn take_user_input() -> String {
    let mut user_input = String::new();
    io::stdin() //get the input from the keyboard, in this case I expect a number
        .read_line(&mut user_input)
        .expect("Error in reading your choice");
    user_input.trim().to_string()
}


fn use_clients(controller: &mut SimulationController){

    println!("\n Which client? \n");

    //Print clients, controller.get_list_clients() function that returns a vec<NodeId>
    let clients_ids = controller.get_list_clients();
    for (i, client) in clients_ids.iter().enumerate(){
        println!(
            "{}- Client with nodeId {} \n", i+1, client.1
        );
    }

    let user_choice = ask_input_user();
    let client_id_chose = clients_ids[(user_choice-1) as usize].1;
    //We should do a check if the id user chose exists!!!

    choose_action_client(client_id_chose, controller);
}

fn choose_action_client(client_id_chose: NodeId, controller: &mut SimulationController) {

    //Variable that allows to go back
    let mut stay_inside = true;
    while stay_inside {

        //Choosing client function
        println!("\n\n Choose client function?");
        println!(
            "1. Start flooding
2. Ask the servers something
3. Go back"
        );
        //2 is to change with more servers

        let user_choice = ask_input_user();


        match user_choice {
            1 => { controller.start_flooding_on_client(client_id_chose).expect("TODO: panic message");}
            2 => {ask_server_action(client_id_chose, controller)}
            3 => { stay_inside = false; }
            _ => println!("Not a valid option, choose again")
        }
    }
}

fn ask_server_action(client_id_chose: NodeId, controller: &mut SimulationController) {

    //!!! To make with more servers

    //Variable that allows to go back
    let mut stay_inside = true;
    while stay_inside {

        //Choosing what to ask servers
        println!("\n\n What is your query?");
        println!(
            "1. Ask type to the servers
2. More
3. Go back"
        );
        let user_choice = ask_input_user();

        match user_choice {
            1 => { controller.ask_server_type_with_client_id(client_id_chose, 4).unwrap() } //For testing it doesn't choose the servers, it's only one with NodeId 4
            2 => println!("to do"),
            3 => { stay_inside = false; }
            _ => println!("Not a valid option, choose again")
        }

    }
}