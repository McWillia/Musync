use ws::{listen, Handler, Sender, Result, Message, Handshake, CloseCode, Error, ErrorKind};
use serde::{Serialize, Deserialize};
use threadpool::ThreadPool;
use std::collections::HashMap;
use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyOAuth, TokenInfo};
use std::sync::{Arc, Mutex};
use std::borrow::Cow;

enum ServiceType {
    MutualPlaylist,
    Other,
}

enum ConnectionType {
    Client,
    Service,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MessageFormat {
    message_type: String,
    string: Option<String>,
    id: Option<u32>,
    client_groups: Option<Vec<ClientGroup>>,
}

#[derive(Debug, Clone)]
struct Client {
    group_id: u32,
    access_token: String,
    expires_at: Option<i64>,
    refresh_token: Option<String>,
    connection: Sender,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClientGroup {
    is_advertising: bool,
    connections: Vec<u32>,
}

struct Server {
    connection: Sender,
    connection_type: Arc<Mutex<ConnectionType>>,
    clients: Arc<Mutex<Vec<Client>>>,
    client_groups: Arc<Mutex<Vec<ClientGroup>>>,
    service_groups: Arc<Mutex<Vec<Vec<Sender>>>>,
    thread_pool: Arc<ThreadPool>,
}

impl Handler for Server {

    fn on_open(&mut self, _: Handshake) -> Result<()> {
        println!("Got new connection: {:?}", self.connection);
        let init = MessageFormat {
            message_type: String::from("initialise"),
            string: None,
            id: connection.connection_id(),
            client_groups: None,
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
        let connection = self.connection.clone();
        let shared_connections = Arc::clone(&self.connections);
        let shared_client_groups = Arc::clone(&self.client_groups);
        let shared_service_groups = Arc::clone(&self.service_groups);
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
                "new_client" => {
                    match new_client(&shared_connections, &shared_client_groups, &shared_client_group_number, &json, connection_id, connection) {
                        Ok(()) => {},
                        Err(error) => {
                            panic!("Couldn't add new client: {:?}", error);
                        }
                    }
                    let mut owned_connection_type = shared_connection_type.lock().unwrap();
                    *owned_connection_type = ConnectionType.Client;
                },
                "get_advertising_client_groups" => {
    
                },
                "get_playlists" => {

                },
                "make_mutual_playlist" => {
                    
                },
                "join_group" => {
                    match join_group(&shared_connections, &shared_client_groups, json, connection_id) {
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
                "new_service" => {
                    match new_service() {
                        Ok(()) => {},
                        Err(error) => {
                            panic!("Couldn't add new service: {:?}", error);
                        }
                    }
                    let mut owned_connection_type = shared_connection_type.lock().unwrap();
                    *owned_connection_type = ConnectionType.Service;
                },
                "result" => {

                },
                &_ => {

                }
            };
        });
        return Ok(());
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("Connection closed: {:?}", self.connection);
        let connection_id = self.connection.connection_id();
        if self.connection_type == ConnectionType.Client {
            let mut owned_connections = self.connections.lock().unwrap();
            let mut owned_client_groups = self.client_groups.lock().unwrap();
            if owned_connections.contains_key(connection_id) {
                remove_client(owned_connections, owned_client_groups, connection_id);
            }
        } else if self.connection_type == ConnectionType.Service {
            let mut owned_connections = self.client_groups.lock().unwrap();
            let mut owned_service_groups = self.service_groups.lock().unwrap();
            if owned_connections.contains_key(connection_id) {
                remove_service(owned_connections, owned_service_groups, connection_id);
            }
        } 
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

fn new_client(shared_connections: &Mutex<HashMap<u32, Client>>, shared_client_groups: &Mutex<HashMap<u32, ClientGroup>>, shared_client_group_number: &Mutex<u32>, json: &MessageFormat, connection_id: u32, connection: Sender) -> Result<()> {
    let auth_code = match &json.code {
        Some(code) => code,
        None => {
            println!("Client didn't specify auth_code in request");
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("No auth_code specified")),
            });
        }
    }
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
    let mut owned_connections = shared_connections.lock().unwrap();
    let mut owned_client_groups = shared_client_groups.lock().unwrap();
    let mut owned_client_group_number = shared_client_group_number.lock().unwrap();
    match insert_client(&owned_connections, &owned_client_groups, &owned_client_group_number, &token, connection_id, connection) {
        Ok(()) => {},
        Err(error) => {
            println!("Error inserting client to HashMaps: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't insert client")),
            });
        },
    };
    match broadcast_client_groups(&owned_connections, &owned_client_groups) {
        Ok(()) => return Ok(()),
        Err(error) => {
            println!("Error broadcasting client update: {:?}", error);
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't broadcast client update")),
            });
        }
    };
}

fn new_service() {
    let service_type = match json.string {
        Some(string) => string,
        None => {
            panic!("Service didn't declare itself");
        },
    };
    let mut owned_connections = shared_connections.lock().unwrap();
    if owned_connections.contains_key(&service_type) {
        let current_connections = owned_connections.get_mut(&service_type);
        let current_connections = match current_connections {
            Some(service) => service,
            None => {
                panic!("Couldn't find current connections array");
            },
        };
        current_connections.push((connection_id, connection));
    } else {
        owned_connections.insert(service_type, vec![(connection_id, connection)]);
    }
}

fn join_group(shared_connections: &Mutex<HashMap<u32, Client>>, shared_client_groups: &Mutex<HashMap<u32, ClientGroup>>, json: MessageFormat, connection_id: u32) -> Result<()> {
    let group_id = match json.id {
        Some(id) => id,
        None => {
            println!("Client didn't specify a group id to join");
                return Err(Error {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(String::from("Couldn't get join request group id")),
                });
        },
    };
    let owned_connections = shared_connections.lock().unwrap();
    let mut owned_client_groups = shared_client_groups.lock().unwrap();
    if !owned_client_groups.contains_key(&group_id) { 
        println!("Client specified nonexistent group");
        return Err(Error {
            kind: ErrorKind::Internal,
            details: Cow::Owned(String::from("Client specified nonexistent group")),
        });
    }
    let current_client = owned_connections.get(&connection_id);
    let current_client = match current_client {
        Some(client) => client,
        None => {
            println!("Client doesn't exist in HashMap");
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Couldn't get current client")),
            });
        },
    };
    let current_group = owned_client_groups.get_mut(&current_client.group_id);
    let current_group = match current_group {
        Some(group) => group,
        None => {
            println!("Current client isn't in a group");
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Client isn't in a group")),
            });
        },
    };
    if current_group.connections.len() == 1 {
        owned_client_groups.remove(&current_client.group_id);
    } else {
        *current_group.connections.remove()
    };
    let destination_group = owned_client_groups.get_mut(&group_id);
    let destination_group = match destination_group {
        Some(group) => group,
        None => {
            println!("Client specified nonexistent group");
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("Client specified nonexistent group")),
            });
        },
    };
    destination_group.connections.push(connection_id);
    broadcast_client_groups(&shared_connections, &shared_client_groups)
}

fn insert_client(owned_connections: &HashMap<u32, Client>, owned_client_groups: &HashMap<u32, ClientGroup>, owned_client_group_number: &u32, token: &TokenInfo, connection_id: u32, connection: Sender) -> Result<()> {
    owned_connections.insert(connection_id, Client {
        group_id: *owned_client_group_number,
        access_token: String::from(&token.access_token),
        expires_at: token.expires_at,
        refresh_token: match &token.refresh_token {
            Some(refresh_token) => Some(String::from(refresh_token)),
            None => None,
        },
        connection: connection,
    });
    let client = vec![connection_id];
    owned_client_groups.insert(*owned_client_group_number, ClientGroup {
        advert: false,
        id: *owned_client_group_number,
        connections: client,
    });
    *owned_client_group_number += 1;
    return Ok(());
}

fn remove_client(owned_connections: &HashMap<u32, Client>, owned_client_groups: &HashMap<u32, ClientGroup>, owned_client_group_number: &u32, connection_id: u32) -> Result<()> {
    let group_id = owned_connections.get(connection_id);

}

fn broadcast_client_groups(shared_connections: &Mutex<HashMap<u32, Client>>, shared_client_groups: &Mutex<HashMap<u32, ClientGroup>>) -> Result<()> {
    let owned_connections = shared_connections.lock().unwrap();
    let owned_client_groups = shared_client_groups.lock().unwrap();
    let message = MessageFormat {
        message_type: String::from("advertising_client_groups"),
        code: None,
        string: None,
        id: None,
        data: Some(owned_client_groups.values().map(|group| ClientGroup {
            advert: group.advert,
            id: group.id,
            connections: group.connections.iter().map(|client| *client).collect(),
        }).collect()),
    };
    let json = serde_json::to_string(&message);
    match json {
        Ok(json) => {
            for (_connection_id, client) in owned_connections.iter() {
                match client.connection.send(String::from(&json)) {
                    Ok(()) => {},
                    Err(error) => {
                        println!("Failed to update client: {:?}", error);
                        return Err(Error {
                            kind: ErrorKind::Internal,
                            details: Cow::Owned(String::from("Couldn't update connections")),
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
async fn get_access_token(new_client: &str) -> Result<TokenInfo> {
    let oauth = SpotifyOAuth::default()
    .client_id("f092792439d74b7e9341f90719b98365")
    .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
    .redirect_uri("http://localhost:3000/home")
    .build();
    let token = oauth.get_access_token(new_client)
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
    let clients = Arc::new<Mutex::new(Vec::new());
    let client_groups = Arc::new(Mutex::new(Vec::new()));
    let service_groups = Arc::new(Mutex::new(Vec::new()));
    let threads = Arc::new(ThreadPool::new(20));
    listen("localhost:8080", |connection| Server {connection: connection, connection_type: Arc::new(Mutex::new(ConnectionType.Unknown)), clients: Arc::clone(&clients), client_groups: Arc::clone(&client_groups), service_groups: Arc::clone(&service_groups), thread_pool: Arc::clone(&threads)}).unwrap();
}