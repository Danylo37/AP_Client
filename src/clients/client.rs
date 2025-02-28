use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use wg_2024::{network::NodeId, packet::Packet};
use crate::general_use::{ClientCommand, ClientEvent, Response};

pub trait Client {
    fn new(
        id: NodeId,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        packet_recv: Receiver<Packet>,
        controller_send: Sender<ClientEvent>,
        controller_recv: Receiver<ClientCommand>,
        ui_response_send: Sender<Response>,
    ) -> Self;

    fn run(&mut self);
}
