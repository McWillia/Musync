use ws::{connect, Handler, Sender, Handshake, Result, Message, CloseCode, Error, ErrorKind};
use rspotify::client::Spotify;
use std::sync::Arc;
use std::borrow::Cow;

use musink::communication::{
    MessageFormat,
    MessageType,
    MessageError,
};
use musink::spotify::*;

struct Client {
    connection: Sender,
}

impl Handler for Client {
    
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        println!("Connected to server");
        let init = MessageFormat {
            message_type: MessageType::NewService,
            id: None,
            strings: Some(vec![String::from("MutualPlaylist")]),
            groups: None,
        };
        let json = serde_json::to_string(&init);
        let json = match json {
            Ok(json) => json,
            Err(error) => {
                panic!("Couldn't convert MessageFormat struct to json: {:?}", error);
            },
        };
        self.connection.send(json)
    }

    fn on_message(&mut self, message: Message) -> Result<()> {
        tokio::spawn(handle_message(Arc::new(message)));
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => {
                println!("Closed normally");
            },
            _ => {
                println!("The client encountered an error: {}", reason);
            },
        };
    }
}

async fn handle_message(message: Arc<Message>) -> Result<()> {
    println!("Message Received");
    let string = match message.as_text() {
        Ok(string) => string,
        Err(error) => return Err(Error{
            kind: ErrorKind::Custom(Box::new(MessageError{})),
            details: Cow::Owned(format!("Couldn't convert message to string: {:?}", error)),
        }),
    };
    let json: MessageFormat = match serde_json::from_str(string){
        Ok(message) => message,
        Err(error) => {
            panic!("Couldn't convert string to InstructMessage struct: {:?}", error);
        },
    };
    match json.message_type {
        MessageType::Initialise => {
            println!("Initialise handshake complete");
        },
        MessageType::MakeMutualPlaylist => {
            let access_tokens = match json.strings {
                Some(token) => token,
                None => return Err(Error{
                    kind: ErrorKind::Custom(Box::new(MessageError{})),
                    details: Cow::Owned(format!("No access tokens were provided")),
                }),
            };
            create_mutual_playlist(&access_tokens).await?;
        },
        _ => {
            return Err(Error {
                kind: ErrorKind::Custom(Box::new(MessageError{})),
                details: Cow::Owned(format!("Unexpected Message Type: {:?}", json))
            });
        },
    };
    println!("Work Done");
    Ok(())
}

async fn create_mutual_playlist(access_tokens: &Vec<String>) -> Result<()> {
    let first_user = Spotify::default()
        .access_token(&(access_tokens[0]))
        .build();
    let second_user = Spotify::default()
        .access_token(&(access_tokens[1]))
        .build();
    let first_tracks = get_user_top_tracks(&first_user).await?;
    let second_tracks = get_user_top_tracks(&second_user).await?;
    let common_tracks = first_tracks.iter().filter_map(|track| match second_tracks.contains(track) {
        true => Some(String::from(track)),
        false => None,
    }).collect::<Vec<String>>();
    let (owner_id, playlist_id) = create_playlist(&first_user, common_tracks).await?;
    follow_playlist(&second_user, &owner_id, &playlist_id).await?;
    Ok(())
}


#[tokio::main]
async fn main() {
    connect("ws://192.168.1.69:8080", |connection| Client {connection: connection}).unwrap();
}