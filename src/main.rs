use crate::network_initializer::NetworkInit;
mod general_use;
mod clients;
mod network_initializer;
mod ui;
mod simulation_controller;
mod servers;
use clients::client_danylo::test_gui::test_gui;

fn main() {
    env_logger::init();

    // test_gui();
    let mut my_net = NetworkInit::new();
    my_net.parse("input.toml")
}
