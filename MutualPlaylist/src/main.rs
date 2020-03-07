// extern crate ws;
// extern crate rspotify;
// extern crate serde;
// extern crate serde_json;

use ws::{connect, Handler, Sender, Handshake, Result, Message, CloseCode};
use rspotify::client::Spotify;
use rspotify::senum::TimeRange;
use rspotify::model::track::FullTrack;
use serde::{Deserialize, Serialize};

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
    access_token: String,
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
        println!("Got access token: {}", message.access_token);
        getUserTopTracks(message.access_token.as_str());
        return Ok(());
    }
}

fn main() {
    connect("ws://138.251.29.21:8082", |connection| Client {connection: connection}).unwrap();
}

#[tokio::main]
async fn getUserTopTracks(access_token: &str) -> Result<Vec<String>> {
    let spotify = Spotify::default()
        .access_token(access_token)
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
            panic!("Couldn't get top tracks: {:?}", error);
        },
    };
}