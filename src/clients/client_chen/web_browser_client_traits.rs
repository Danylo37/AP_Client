use crate::clients::client_chen::ClientChen;
use crate::general_use::{MediaRef, Response};

pub trait WebBrowserClientTrait {
    fn handle_list_file(&mut self, list_file: Vec<String>);
    fn handle_text_file(&mut self, text_file: String);
    fn handle_media(&mut self, serialized_media: String);
}

impl WebBrowserClientTrait for ClientChen{
    fn handle_list_file(&mut self, list_file: Vec<String>) {
        //just update the list of file
        self.storage.current_list_file = list_file.clone();     // todo remove clone

        self.ui_response_send.send(Response::ListFiles(list_file)).unwrap()     // todo remove this line
    }

   fn handle_text_file(&mut self, text_file: String) {
       self.storage.current_requested_text_file = String::from(text_file.clone());

       let media_refs = filter_media_refs_from_text(text_file.clone());     // todo remove clone
       self.storage.current_text_media_list = media_refs;

       self.ui_response_send.send(Response::File(text_file)).unwrap()     // todo remove this line
   }

    fn handle_media(&mut self, serialized_media: String) {
        self.storage.current_received_serialized_media.insert(self.storage.current_chosen_media_ref.clone(), serialized_media.clone());     // todo remove this line

        self.ui_response_send.send(Response::Media(serialized_media)).unwrap()     // todo remove this line

    }
}

pub fn filter_media_refs_from_text(input: String) -> Vec<MediaRef> {
    input
        .split_whitespace()
        .filter(|word| {
            // Ensure minimum length to avoid panics
            word.len() >= 8 &&
                // Check prefix "#Media["
                word.starts_with("#Media[") &&
                // Check suffix "]"
                word.ends_with(']') &&
                // Ensure no inner ']' between "#Media[" and the closing "]"
                !word[7..word.len() - 1].contains(']')
        })
        .map(|word| word.to_string()) // Convert &str to String
        .collect()
}