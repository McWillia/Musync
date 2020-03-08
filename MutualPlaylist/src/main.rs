// extern crate ws;
// extern crate rspotify;
// extern crate serde;
// extern crate serde_json;

use ws::{connect, Handler, Sender, Handshake, Result, Message, Error, ErrorKind, CloseCode};
use rspotify::client::Spotify;
use rspotify::senum::TimeRange;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use threadpool::ThreadPool;
// use serde_json::Result;

struct Client {
    connection: Sender,
    thread_pool: ThreadPool,
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
        let con = self.connection.clone();
        ctrlc::set_handler(move || {
            match con.close(CloseCode::Normal){
                Ok(_) => println!("Got close command"),
                Err(error) => println!("Error closing socket: {}", error),
            };
        }).expect("Error setting Ctrl-C handler");
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
        self.thread_pool.execute(move || {
            println!("Message Received");
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
            match create_mutual_playlist(message.access_tokens.iter().map(|access_token| String::from(access_token)).collect()) {
                Ok(()) => println!("Work complete"),
                Err(error) => println!("Couldn't create mutual playlist: {}", error),
            };
        });
        return Ok(());
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
        self.thread_pool.join();
        println!("All threads joined");
    }
}

fn main() {
    connect("ws://138.251.29.150:8082", |connection| Client {connection: connection, thread_pool: ThreadPool::new(20)}).unwrap();
}

#[tokio::main]
async fn create_mutual_playlist(access_tokens: Vec<String>) -> Result<()> {
    let first_user = Spotify::default()
        .access_token(&(access_tokens[0]))
        .build();
    let second_user = Spotify::default()
        .access_token(&(access_tokens[1]))
        .build();
    let first_tracks = get_user_top_tracks(&first_user).await;
    let first_tracks = match first_tracks {
        Ok(tracks) => tracks,
        Err(error) => {
            panic!("Couldn't get top tracks: {:?}", error);
        },
    };
    let second_tracks = get_user_top_tracks(&second_user).await;
    let second_tracks = match second_tracks {
        Ok(tracks) => tracks,
        Err(error) => {
            panic!("Couldn't get top tracks: {:?}", error);
        },
    };
    let common_tracks = first_tracks.iter().filter_map(|track| match second_tracks.contains(track) {
        true => Some(String::from(track)),
        false => None,
    }).collect::<Vec<String>>();
    let result = create_playlist(&first_user, common_tracks).await;
    let (owner_id, playlist_id) = match result {
        Ok(result) => result,
        Err(error) => {
            panic!("Couldn't get top tracks: {:?}", error);
        },
    };
    let result = follow_playlist(&second_user, &owner_id, &playlist_id).await;
    return result;
}

async fn get_user_top_tracks(spotify: &Spotify) -> Result<Vec<String>> {
    let mut ids: Vec<String> = Vec::new();
    for time_range in [TimeRange::ShortTerm, TimeRange::MediumTerm, TimeRange::LongTerm].iter() {
        let result = spotify
            .current_user_top_tracks(50, 0, *time_range)
            .await;
        match result {
            Ok(tracks) => {
                ids.append(&mut tracks.items.iter().filter_map(|track| match &track.id {
                    Some(id) => Some(String::from(id)),
                    None => None,
                }).collect::<Vec<String>>());
            },
            Err(error) => {
                println!("Error getting top tracks: {:?}", error);
                return Err(Error {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(String::from("Couldn't get top tracks")),
                });
            },
        };
    }
    return Ok(ids);
}

async fn create_playlist(spotify: &Spotify, common_tracks: Vec<String>) -> Result<(String, String)> {
    let user = spotify
        .current_user()
        .await;
    let user = match user {
        Ok(user) => user,
        Err(error) => {
            println!("Error getting current user: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't get current user")),
            });
        },
    };
    let result = spotify
        .user_playlist_create(&user.id, "MutualPlaylist", Some(false), Some(String::from("MutualPlaylist")))
        .await;
    let playlist = match result {
        Ok(playlist) => playlist,
        Err(error) => {
            println!("Error creating playlist: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't create playlist")),
            });
        },
    };
    for x in (0..common_tracks.len()).step_by(20) {
        let slice;
        if common_tracks.len() > x + 20 {
            slice = &common_tracks[x..x+20];
        } else {
            slice = &common_tracks[x..];
        }
        let result = spotify
            .user_playlist_add_tracks(&user.id, &playlist.id, slice, None)
            .await;
        match result.err() {
            Some(error) => {
                println!("Error adding tracks to playlist: {:?}", error);
                return Err(Error {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(String::from("Couldn't add tracks to playlist")),
                });
            },
            None => {},
        };
    }
    let result = spotify
        .user_playlist_change_detail(&user.id, &playlist.id, None, None, None, Some(true))
        .await;
    match result {
        Ok(_) => return Ok((user.id, playlist.id)),
        Err(error) => {
            println!("Error making playlist collaborative: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't make playlist collaborative")),
            });
        },
    };
}

async fn follow_playlist(spotify: &Spotify, owner_id: &str, playlist_id: &str) -> Result<()> {
    let result = spotify
        .user_playlist_follow_playlist(&owner_id, &playlist_id, None)
        .await;
    match result {
        Ok(_) => return Ok(()),
        Err(error) => {
            println!("Error following playlist: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't follow playlist")),
            });
        },
    }
}