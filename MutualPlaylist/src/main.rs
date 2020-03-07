// extern crate ws;
// extern crate rspotify;
// extern crate serde;
// extern crate serde_json;

use ws::{connect, Handler, Sender, Handshake, Result, Message, Error, ErrorKind, CloseCode};
use rspotify::client::Spotify;
use rspotify::senum::TimeRange;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
// use serde_json::Result;

struct Client {
    connection: Sender,
}

#[derive(Serialize, Deserialize)]
struct InitMessage {
    r#type: String,
    microservice_type: String,
}

// #[derive(Serialize, Deserialize)]
// struct ResultMessage {
//     r#type: String,
//     result: FullTrack[],
// }

#[derive(Serialize, Deserialize)]
struct InstructMessage {
    access_tokens: Vec<String>,
}

impl Handler for Client {
    
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        println!("Connected to server");
        let init = InitMessage {
            r#type: String::from("new"),
            microservice_type: String::from("MutualPlaylist"),
        };
        let json = serde_json::to_string(&init);
        let json = match json {
            Ok(json) => json,
            Err(error) => {
                panic!("Couldn't convert InitMessage struct to json: {:?}", error);
            },
        };
        return self.connection.send(json);
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let string = msg.as_text();
        let string = match string {
            Ok(string) => string,
            Err(error) => {
                panic!("Couldn't convert WebSocket message to string: {:?}", error);
            },
        };
        let message = serde_json::from_str(string);
        let message: InstructMessage = match message {
            Ok(message) => message,
            Err(error) => {
                panic!("Couldn't convert string to InstructMessage struct: {:?}", error);
            },
        };
        println!("Got access token: {:?}", message.access_tokens);
        createMutualPlaylist(message.access_tokens.iter().map(|access_token| String::from(access_token)).collect());
        return Ok(());
    }
}

fn main() {
    connect("ws://138.251.29.21:8082", |connection| Client {connection: connection}).unwrap();
}

#[tokio::main]
async fn createMutualPlaylist(access_tokens: Vec<String>) -> Result<()> {
    let first_tracks = getUserTopTracks(String::from(access_tokens[0].as_str())).await;
    let first_tracks = match first_tracks {
        Ok(tracks) => tracks,
        Err(error) => {
            panic!("Couldn't get top tracks: {:?}", error);
        },
    };
    println!("First Tracks: {:?}", first_tracks);
    let second_tracks = getUserTopTracks(String::from(access_tokens[1].as_str())).await;
    let second_tracks = match second_tracks {
        Ok(tracks) => tracks,
        Err(error) => {
            panic!("Couldn't get top tracks: {:?}", error);
        },
    };
    println!("Second Tracks: {:?}", second_tracks);
    return Ok(());
}

async fn getUserTopTracks(access_token: String) -> Result<Vec<String>> {
    let spotify = Spotify::default()
        .access_token(access_token.as_str())
        .build();
    let tracks = spotify
        .current_user_top_tracks(50, 0, TimeRange::MediumTerm)
        .await;
    match tracks {
        Ok(tracks) => {
            let ids = tracks.items.iter().map(|track| String::from(track.id.as_ref().unwrap()) ).collect::<Vec<String>>();
            return Ok(ids);
        },
        Err(error) => {
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from(error.name().unwrap())),
            });
        },
    };
}