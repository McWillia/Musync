// extern crate ws;
// extern crate rspotify;
// extern crate serde;
// extern crate serde_json;

use ws::{connect, Handler, Sender, Handshake, Result, Message, Error, ErrorKind/*, CloseCode*/};
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
        println!("Connected to server");
        self.thread_pool = ThreadPool::new(20);
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
            createMutualPlaylist(message.access_tokens.iter().map(|access_token| String::from(access_token)).collect());
        });
        return Ok(());
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => {
                println!("Closed normally");
                self.thread_pool.join();
                println!("All threads joined");
            },
            _ => println!("The client encountered an error: {}", reason);
        }
    }
}

fn main() {
    connect("ws://138.251.29.159:8082", |connection| Client {connection: connection}).unwrap();
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
    let second_tracks = getUserTopTracks(String::from(access_tokens[1].as_str())).await;
    let second_tracks = match second_tracks {
        Ok(tracks) => tracks,
        Err(error) => {
            panic!("Couldn't get top tracks: {:?}", error);
        },
    };
    let common_tracks = first_tracks.iter().filter_map(|track| match second_tracks.contains(track) {
        true => Some(String::from(track.as_str())),
        false => None,
    }).collect::<Vec<String>>();
    let result = createPlaylist(String::from(access_tokens[0].as_str()), common_tracks).await;
    let (owner_id, playlist_id) = match result {
        Ok(result) => result,
        Err(error) => {
            panic!("Couldn't get top tracks: {:?}", error);
        },
    };
    let result = followPlaylist(String::from(access_tokens[1].as_str()), owner_id, playlist_id).await;
    return result;
}

async fn getUserTopTracks(access_token: String) -> Result<Vec<String>> {
    let spotify = Spotify::default()
        .access_token(access_token.as_str())
        .build();
    let mut ids: Vec<String> = Vec::new();
    for time_range in [TimeRange::ShortTerm, TimeRange::MediumTerm, TimeRange::LongTerm].iter() {
        let tracks = spotify
            .current_user_top_tracks(50, 0, *time_range)
            .await;
            match tracks {
                Ok(tracks) => {
                    ids.append(&mut tracks.items.iter().filter_map(|track| match &track.id {
                    Some(id) => Some(String::from(id)),
                    None => None,
                }).collect::<Vec<String>>());
            },
            Err(error) => {
                println!("Error: {:?}", error);
                return Err(Error {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(String::from("Couldn't get top tracks")),
                });
            },
        };
    }
    return Ok(ids);
}

async fn createPlaylist(access_token: String, common_tracks: Vec<String>) -> Result<(String, String)> {
    let spotify = Spotify::default()
        .access_token(access_token.as_str())
        .build();
    let user = spotify
        .current_user()
        .await;
    let user = match user {
        Ok(user) => user,
        Err(error) => {
            println!("Error: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't get current user")),
            });
        },
    };
    let playlist = spotify
        .user_playlist_create(&user.id, "MutualPlaylist", Some(false), Some(String::from("MutualPlaylist")))
        .await;
    let playlist = match playlist {
        Ok(playlist) => playlist,
        Err(error) => {
            println!("Error: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't create playlist")),
            });
        },
    };
    for x in (0..common_tracks.len()).step_by(20) {
        let slice;
        if (common_tracks.len() > x + 20) {
            slice = &common_tracks[x..x+20];
        } else {
            slice = &common_tracks[x..];
        }
        let result = spotify
            .user_playlist_add_tracks(&user.id, &playlist.id, slice, None)
            .await;
        result.or_else(|error| {
            println!("Error: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't add tracks to playlist")),
            });
        });
    }
    let modification = spotify
        .user_playlist_change_detail(&user.id, &playlist.id, None, None, None, Some(true))
        .await;
    match modification {
        Ok(modification) => return Ok((user.id, playlist.id)),
        Err(error) => {
            println!("Error: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't make playlist collaborative")),
            });
        },
    };
}

async fn followPlaylist(access_token: String, owner_id: String, playlist_id: String) -> Result<()> {
    let spotify = Spotify::default()
        .access_token(access_token.as_str())
        .build();
    let result = spotify
        .user_playlist_follow_playlist(&owner_id, &playlist_id, None)
        .await;
    match result {
        Ok(result) => return Ok(()),
        Err(error) => {
            println!("Error: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't follow playlist")),
            });
        },
    }
}