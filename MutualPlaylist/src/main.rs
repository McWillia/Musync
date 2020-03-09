use ws::{connect, Handler, Sender, Handshake, Result, Message, Error, ErrorKind, CloseCode};
use rspotify::client::Spotify;
use rspotify::senum::TimeRange;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use threadpool::ThreadPool;

struct Client {
    connection: Sender,
    thread_pool: ThreadPool,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum MessageType {
    Initialise,
    NewClient,
    NewService,
    AdvertisingClientGroups,
    GetPlaylists,
    MakeMutualPlaylist,
    JoinGroup,
    Pause,
    Play,
    AddToQueue,
}

#[derive(Serialize, Deserialize)]
struct MessageFormat {
    message_type: MessageType,
    id: Option<u32>,
    strings: Option<Vec<String>>,
    //data: Option<Vec<ClientGroup>>,
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
        let init = MessageFormat {
            message_type: MessageType::NewService,
            id: None,
            strings: Some(vec![String::from("MutualPlaylist")]),
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
                        None => {
                            panic!("No access tokens were provided");
                        },
                    };
                    match create_mutual_playlist(&access_tokens) {
                        Ok(()) => println!("Work complete"),
                        Err(error) => println!("Couldn't create mutual playlist: {}", error),
                    };
                },
                _ => {

                },
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


#[tokio::main]
async fn create_mutual_playlist(access_tokens: &Vec<String>) -> Result<()> {
    let first_user = Spotify::default()
        .access_token(&(access_tokens[0]))
        .build();
    let second_user = Spotify::default()
        .access_token(&(access_tokens[1]))
        .build();
    let first_tracks = get_user_top_tracks(&first_user).await.expect("Couldn't get user top tracks");
    let second_tracks = get_user_top_tracks(&second_user).await.expect("Couldn't get user top tracks");
    let common_tracks = first_tracks.iter().filter_map(|track| match second_tracks.contains(track) {
        true => Some(String::from(track)),
        false => None,
    }).collect::<Vec<String>>();
    let (owner_id, playlist_id) = create_playlist(&first_user, common_tracks).await.expect("Couldn't create playlist");
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

fn main() {
    connect("ws://192.168.1.69:8080", |connection| Client {connection: connection, thread_pool: ThreadPool::new(20)}).unwrap();
}