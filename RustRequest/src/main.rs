use ws::{listen, Handler, Sender, Result, Message, Handshake, CloseCode, Error, ErrorKind};
use serde::{Serialize, Deserialize};
use threadpool::ThreadPool;
use std::collections::HashMap;
use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyOAuth, TokenInfo};
use std::sync::{Arc, Mutex};
use std::borrow::Cow;

#[derive(Debug, Serialize, Deserialize)]
struct MessageFormat {
    message_type: String,
    string: Option<String>,
    code: Option<String>,
    id: Option<u32>,
    data: Option<Vec<Group>>,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct User {
    group_id: u32,
    access_token: String,
    expires_at: Option<i64>,
    refresh_token: Option<String>,
    connection: Sender,
}

#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
struct Group {
    advert: bool,
    id: u32,
    clients: Vec<u32>,
}

struct Server {
    connection: Sender,
    users: Arc<Mutex<HashMap<u32, User>>>,
    groups: Arc<Mutex<HashMap<u32, Group>>>,
    services: Arc<Mutex<HashMap<String, Vec<(u32, Sender)>>>>,
    group_number: Arc<Mutex<u32>>,
    thread_pool: Arc<ThreadPool>,
}

impl Handler for Server {

    fn on_open(&mut self, _: Handshake) -> Result<()> {
        println!("Got new connection: {:?}", self.connection);
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let connection = self.connection.clone();
        let shared_users = Arc::clone(&self.users);
        let shared_groups = Arc::clone(&self.groups);
        let shared_services = Arc::clone(&self.services);
        let shared_group_number = Arc::clone(&self.group_number);
        let message = msg.clone();
        self.thread_pool.execute(move || {
            let connection_id = connection.connection_id();
            let text = message.as_text();
            let text = match text {
                Ok(text) => text,
                Err(error) => {
                    panic!("Message isn't in text format: {:?}", error);
                },
            };
            let json: MessageFormat = match serde_json::from_str(text) {
                Ok(json) => json,
                Err(error) => {
                    panic!("Error converting message to json: {:?} {:?}", error, message);
                },
            };
            println!("Got message: \ntext = {:?} \n json = {:?}", message, json);
            match json.message_type.as_str() {
                "authCode" => {
                    match auth_code(&shared_users, &shared_groups, &shared_group_number, &json, connection_id, connection) {
                        Ok(()) => {},
                        Err(error) => {
                            panic!("Couldn't get auth code: {:?}", error);
                        }
                    }
                },
                "get_advertising_groups" => {
    
                },
                "get_playlists" => {

                },
                "make_mutual_playlist" => {
                    
                },
                "join_group" => {
                    match join_group(&shared_users, &shared_groups, json, connection_id) {
                        Ok(()) => {},
                        Err(error) => {
                            panic!("Couldn't join group: {:?}", error);
                        }
                    };
                },
                "pause" => {

                },
                "play" => {

                },
                "add_to_queue" => {

                },
                "new" => {
                    let service_type = match json.string {
                        Some(string) => string,
                        None => {
                            panic!("Service didn't declare itself");
                        },
                    };
                    let mut owned_services = shared_services.lock().unwrap();
                    if owned_services.contains_key(&service_type) {
                        let current_services = owned_services.get_mut(&service_type);
                        let current_services = match current_services {
                            Some(service) => service,
                            None => {
                                panic!("Couldn't find current services array");
                            },
                        };
                        current_services.push((connection_id, connection));
                    } else {
                        owned_services.insert(service_type, vec![(connection_id, connection)]);
                    }
                },
                "result" => {

                },
                &_ => {

                }
            };
        });
        // self.connection.send(msg)
        return Ok(());
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("Connection closed: {:?}", self.connection);
        match code {
            CloseCode::Normal => println!("The client closed the connection."),
            CloseCode::Away   => println!("The client left the site."),
            CloseCode::Abnormal => println!("Closing handshake failed! Unable to obtain closing status from client."),
            _ => println!("The client encountered an error: {}", reason),
        }
    }

    fn on_error(&mut self, err: Error) {
        println!("The server encountered an error: {:?}", err);
    }
}

fn auth_code(shared_users: &Mutex<HashMap<u32, User>>, shared_groups: &Mutex<HashMap<u32, Group>>, shared_group_number: &Mutex<u32>, json: &MessageFormat, connection_id: u32, connection: Sender) -> Result<()> {
    match &json.code {
        Some(code) => {
            let token = get_access_token(&code);
            let token = match token {
                Ok(token) => token,
                Err(error) => {
                    println!("Error getting access token: {:?}", error);
                    return Err(Error {
                        kind: ErrorKind::Internal,
                        details: Cow::Owned(String::from("Couldn't get access token")),
                    });
                },
            };
            // println!("Retrieved Access Token: {:?}", token);
            match insert_user(&shared_users, &shared_groups, &shared_group_number, &token, connection_id, connection) {
                Ok(()) => {},
                Err(error) => {
                    println!("Error inserting user to HashMaps: {:?}", error);
                    return Err(Error {
                        kind: ErrorKind::Internal,
                        details: Cow::Owned(String::from("Couldn't insert user")),
                    });
                },
            };
            match update_groups(&shared_users, &shared_groups) {
                Ok(()) => return Ok(()),
                Err(error) => {
                    println!("Error broadcasting user update: {:?}", error);
                    return Err(Error {
                        kind: ErrorKind::Internal,
                        details: Cow::Owned(String::from("Couldn't broadcast user update")),
                    });
                }
            };
        },
        None => {
            println!("User didn't specify auth_code in request");
                    return Err(Error {
                        kind: ErrorKind::Internal,
                        details: Cow::Owned(String::from("No auth_code specified")),
                    });
        }
    }
}

fn join_group(shared_users: &Mutex<HashMap<u32, User>>, shared_groups: &Mutex<HashMap<u32, Group>>, json: MessageFormat, connection_id: u32) -> Result<()> {
    let group_id = match json.id {
        Some(id) => id,
        None => {
            println!("User didn't specify a group id to join");
                return Err(Error {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(String::from("Couldn't get join request group id")),
                });
        },
    };
    let owned_users = shared_users.lock().unwrap();
    let mut owned_groups = shared_groups.lock().unwrap();
    if owned_groups.contains_key(&group_id) {
        let current_user = owned_users.get(&connection_id);
        let current_user = match current_user {
            Some(user) => user,
            None => {
                println!("User doesn't exist in HashMap");
                return Err(Error {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(String::from("Couldn't get current user")),
                });
            },
        };
        let current_group = owned_groups.get_mut(&current_user.group_id);
        let current_group = match current_group {
            Some(group) => group,
            None => {
                println!("Current user isn't in a group");
                return Err(Error {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(String::from("User isn't in a group")),
                });
            },
        };
        if current_group.clients.len() == 1 {
            owned_groups.remove(&current_user.group_id);
        } else {
            current_group.clients = current_group.clients.iter().filter_map(|client| match *client != connection_id {
                true => Some(*client),
                false => None,
            }).collect();
        };
        let destination_group = owned_groups.get_mut(&group_id);
        let destination_group = match destination_group {
            Some(group) => group,
            None => {
                println!("User specified nonexistent group");
                return Err(Error {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(String::from("User specified nonexistent group")),
                });
            },
        };
        destination_group.clients.push(connection_id);
        update_groups(&shared_users, &shared_groups)
    } else {
        println!("User specified nonexistent group");
        return Err(Error {
            kind: ErrorKind::Internal,
            details: Cow::Owned(String::from("User specified nonexistent group")),
        });
    }
}

fn insert_user(shared_users: &Mutex<HashMap<u32, User>>, shared_groups: &Mutex<HashMap<u32, Group>>, shared_group_number: &Mutex<u32>, token: &TokenInfo, connection_id: u32, connection: Sender) -> Result<()> {
    let mut owned_users = shared_users.lock().unwrap();
    let mut owned_groups = shared_groups.lock().unwrap();
    let mut owned_group_number = shared_group_number.lock().unwrap();
    owned_users.insert(connection_id, User {
        group_id: *owned_group_number,
        access_token: String::from(&token.access_token),
        expires_at: token.expires_at,
        refresh_token: match &token.refresh_token {
            Some(refresh_token) => Some(String::from(refresh_token)),
            None => None,
        },
        connection: connection,
    });
    let user = vec![connection_id];
    owned_groups.insert(*owned_group_number, Group {
        advert: false,
        id: *owned_group_number,
        clients: user,
    });
    *owned_group_number += 1;
    return Ok(());
}

fn update_groups(shared_users: &Mutex<HashMap<u32, User>>, shared_groups: &Mutex<HashMap<u32, Group>>) -> Result<()> {
    let owned_users = shared_users.lock().unwrap();
    let owned_groups = shared_groups.lock().unwrap();
    let message = MessageFormat {
        message_type: String::from("advertising_groups"),
        code: None,
        string: None,
        id: None,
        data: Some(owned_groups.values().map(|group| Group {
            advert: group.advert,
            id: group.id,
            clients: group.clients.iter().map(|client| *client).collect(),
        }).collect()),
    };
    let json = serde_json::to_string(&message);
    match json {
        Ok(json) => {
            for (_connection_id, user) in owned_users.iter() {
                match user.connection.send(String::from(&json)) {
                    Ok(()) => {},
                    Err(error) => {
                        println!("Failed to update user: {:?}", error);
                        return Err(Error {
                            kind: ErrorKind::Internal,
                            details: Cow::Owned(String::from("Couldn't update users")),
                        });
                    }
                }
            };
        },
        Err(error) => {
            println!("Error converting MessageFormat object to json string: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't convert MessageFormat object to json string")),
            });
        },
    };
    return Ok(());
}

#[tokio::main]
async fn get_access_token(auth_code: &str) -> Result<TokenInfo> {
    let oauth = SpotifyOAuth::default()
    .client_id("f092792439d74b7e9341f90719b98365")
    .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
    .redirect_uri("http://localhost:3000/home")
    .build();
    let token = oauth.get_access_token(auth_code)
    .await;
    let token = match token {
        Some(token) => token,
        None => {
            println!("Error getting access token");
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't get access token")),
            });
        }
    };
    return Ok(token);
}

fn main() {
    let users = Arc::new(Mutex::new(HashMap::new()));
    let groups = Arc::new(Mutex::new(HashMap::new()));
    let services = Arc::new(Mutex::new(HashMap::new()));
    let group_number = Arc::new(Mutex::new(0));
    let threads = Arc::new(ThreadPool::new(20));
    listen("localhost:8080", |connection| Server {connection: connection, users: Arc::clone(&users), groups: Arc::clone(&groups), services: Arc::clone(&services), group_number: Arc::clone(&group_number), thread_pool: Arc::clone(&threads)}).unwrap();
}