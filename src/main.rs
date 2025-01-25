use crate::network_initializer::NetworkInit;
mod general_use;
mod clients;
mod network_initializer;
mod ui;
mod simulation_controller;
mod servers;
mod new_ui_test;


fn main() {
    env_logger::init();

    let mut my_net = NetworkInit::new();
    // my_net.parse("input.toml");
    my_net.parse("topologies/butterfly.toml");
}
