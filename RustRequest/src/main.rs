use ws::{listen, Handler, Sender, Result, Message, Handshake, CloseCode, Error, ErrorKind};
use serde::{Serialize, Deserialize};
use threadpool::ThreadPool;
use std::collections::HashMap;
use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyOAuth, TokenInfo};
use std::sync::{Arc, Mutex};
use std::borrow::Cow;
use std::time::{Instant, Duration};

#[derive(Clone, Copy)]
enum ServiceType {
    MutualPlaylist,
    Other,
    Unknown,
}

#[derive(Clone, Copy)]
enum ConnectionType {
    Client,
    Service(ServiceType),
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MessageFormat {
    message_type: MessageType,
    id: Option<usize>,
    strings: Option<Vec<String>>,
    groups: Option<Vec<ClientGroup>>,
}

#[derive(Debug, Clone)]
struct Client {
    group_id: usize,
    access_token: String,
    expires_at: Instant,
    refresh_token: String,
    connection: Sender,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClientGroup {
    group_id: usize,
    is_advertising: bool,
    clients: Vec<u32>,
}

struct Server {
    connection: Sender,
    connection_type: Arc<Mutex<ConnectionType>>,
    clients: Arc<Mutex<HashMap<u32, Client>>>,
    client_groups: Arc<Mutex<HashMap<usize, ClientGroup>>>,
    client_group_count: Arc<Mutex<usize>>,
    service_groups: Arc<Mutex<[Vec<Sender>; 2]>>,
    thread_pool: Arc<ThreadPool>,
}

impl Handler for Server {

    fn on_open(&mut self, _: Handshake) -> Result<()> {
        println!("Got new connection: {:?}", self.connection);
        let init = MessageFormat {
            message_type: MessageType::Initialise,
            id: Some(self.connection.connection_id() as usize),
            strings: None,
            groups: None,
        };
        let json = match serde_json::to_string(&init) {
            Ok(json) => json,
            Err(error) => {
                println!("Couldn't convert json to string: {:?}", error);
                return Ok(());
            },
        };
        self.connection.send(json)
    }

    fn on_message(&mut self, message: Message) -> Result<()> {
        let connection = self.connection.clone();
        let shared_connection_type = Arc::clone(&self.connection_type); //Just shared within threads spawned from this connection
        let shared_clients = Arc::clone(&self.clients); //Shared by all connections
        let shared_client_groups = Arc::clone(&self.client_groups); //Shared by all connections
        let shared_client_group_count = Arc::clone(&self.client_group_count);
        let shared_service_groups = Arc::clone(&self.service_groups); //Shared by all connections
        self.thread_pool.execute(move || {
            let text = match message.as_text() {
                Ok(text) => text,
                Err(error) => {
                    println!("Message wasn't in string form: {:?}", error);
                    return;
                },
            };
            let json: MessageFormat = match serde_json::from_str(text){
                Ok(json) => json,
                Err(error) => {
                    println!("Couldn't parse string to json: {:?}", error);
                    return;
                },
            };
            println!("Got message: \ntext = {:?} \n json = {:?}", message, json);
            match json.message_type {
                MessageType::NewClient => {
                    new_client(&shared_clients, &shared_client_groups, &shared_client_group_count, &json, &connection).expect("Could't make new client");
                    let mut owned_connection_type = shared_connection_type.lock().unwrap();
                    *owned_connection_type = ConnectionType::Client;
                },
                MessageType::MakeMutualPlaylist => {
                    make_mutual_playlist(&shared_clients, &shared_client_groups, &shared_service_groups, &connection).expect("Couldn't make mutual playlist");
                },
                MessageType::JoinGroup => {
                    join_group(&shared_clients, &shared_client_groups, &json, &connection).expect("Couldn't join group");
                },
                MessageType::NewService => {
                    match new_service(&shared_service_groups, &json, &connection) {
                        Ok(service_type) => {
                            let mut owned_connection_type = shared_connection_type.lock().unwrap();
                            *owned_connection_type = ConnectionType::Service(service_type);
                        },
                        Err(error) => {
                            panic!("Couldn't add new service: {:?}", error);
                        },
                    };
                },
                MessageType::Pause => {
                    pause(&shared_clients, &shared_client_groups, &connection).expect("Couldn't pause all users");
                },
                MessageType::Play => {
                    play(&shared_clients, &shared_client_groups, &connection).expect("Couldn't pause all users");
                }
                _ => {

                },
            };
        });
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("Connection closed: {:?}", self.connection);
        let owned_connection_type = self.connection_type.lock().unwrap();
        match *owned_connection_type {
            ConnectionType::Client => {
                let owned_clients = self.clients.lock().unwrap();
                let ref mut owned_client_groups = self.client_groups.lock().unwrap();
                if owned_clients.contains_key(&self.connection.connection_id()) {
                    remove_client(&owned_clients, owned_client_groups, &self.connection).expect("Couldn't remove client");
                    broadcast_client_groups(&owned_clients, owned_client_groups).expect("Couldn't broadcast client groups");
                };
            },
            ConnectionType::Service(service_type) => {
                let mut owned_service_groups = self.service_groups.lock().unwrap();
                let group_index = match service_type {
                    ServiceType::MutualPlaylist => 0,
                    ServiceType::Other => 1,
                    ServiceType::Unknown => {
                        println!("Unknown service closed");
                        return;
                    }
                };
                let service_index_in_group = owned_service_groups[group_index].iter().position(|service| *service == self.connection);
                let service_index_in_group = match service_index_in_group {
                    Some(index) => index,
                    None =>  {
                        println!("Service connection {:?} doesn't exist in service group list", self.connection);
                        return;
                    },
                };
                owned_service_groups[group_index].remove(service_index_in_group);
            },
            ConnectionType::Unknown => {
                println!("Connection closed that hadn't done anything");
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

fn pause(shared_clients: &Mutex<HashMap<u32, Client>>, shared_client_groups: &Mutex<HashMap<usize, ClientGroup>>, connection: &Sender) -> Result<()> {
    let mut owned_clients = shared_clients.lock().unwrap();
    let owned_client_groups = shared_client_groups.lock().unwrap();
    let current_client = match owned_clients.get(&connection.connection_id()) {
        Some(client) => client,
        None => {
            println!("Couldn't find client in map");
            return Ok(());
        },
    };
    let current_group = match owned_client_groups.get(&current_client.group_id) {
        Some(group) => group,
        None => {
            println!("Client belongs to nonexistent group");
            return Ok(());
        },
    };
    for client_id in current_group.clients.iter() {
        let client = match owned_clients.get_mut(client_id) {
            Some(client) => client,
            None => {
                println!("Client {} doesn't exist in map", client_id);
                continue
            },
        };
        if Instant::now() > client.expires_at {
            let new_token = match refresh_token(&client.access_token) {
                Ok(token) => token,
                Err(error) => {
                    println!("Error refreshing access token: {:?}", error);
                    return Ok(());
                },
            };
            client.access_token = new_token.access_token;
            match new_token.refresh_token {
                Some(refresh_token) => client.refresh_token = refresh_token,
                None => {},
            };
        }
        spotify_pause(&client.access_token).expect("Couldn't pause all clients");
    }
    Ok(())
}

fn play(shared_clients: &Mutex<HashMap<u32, Client>>, shared_client_groups: &Mutex<HashMap<usize, ClientGroup>>, connection: &Sender) -> Result<()> {
    let mut owned_clients = shared_clients.lock().unwrap();
    let owned_client_groups = shared_client_groups.lock().unwrap();
    let current_client = match owned_clients.get(&connection.connection_id()) {
        Some(client) => client,
        None => {
            println!("Couldn't find client in map");
            return Ok(());
        },
    };
    let current_group = match owned_client_groups.get(&current_client.group_id) {
        Some(group) => group,
        None => {
            println!("Client belongs to nonexistent group");
            return Ok(());
        },
    };
    for client_id in current_group.clients.iter() {
        let client = match owned_clients.get_mut(client_id) {
            Some(client) => client,
            None => {
                println!("Client {} doesn't exist in map", client_id);
                continue
            },
        };
        if Instant::now() > client.expires_at {
            let new_token = match refresh_token(&client.access_token) {
                Ok(token) => token,
                Err(error) => {
                    println!("Error refreshing access token: {:?}", error);
                    return Ok(());
                },
            };
            client.access_token = new_token.access_token;
            match new_token.refresh_token {
                Some(refresh_token) => client.refresh_token = refresh_token,
                None => {},
            };
        }
        spotify_play(&client.access_token).expect("Couldn't pause all clients");
    }
    Ok(())
}

fn new_client(shared_clients: &Mutex<HashMap<u32, Client>>, shared_client_groups: &Mutex<HashMap<usize, ClientGroup>>, shared_client_group_count: &Mutex<usize>, json: &MessageFormat, connection: &Sender) -> Result<()> {
    let auth_code = match &json.strings {
        Some(code) => &code[0],
        None => {
            println!("Client didn't specify auth_code in request");
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::Owned(String::from("No auth_code specified")),
            });
        },
    };
    let token = spotify_get_access_token(&auth_code).expect("Couldn't get access token");
    let ref mut owned_clients = shared_clients.lock().unwrap();
    let ref mut owned_client_groups = shared_client_groups.lock().unwrap();
    add_new_client(owned_clients, owned_client_groups, shared_client_group_count, &token, connection).expect("Couldn't add new client");
    broadcast_client_groups(&owned_clients, &owned_client_groups).expect("Couldn't broadcast client groups");
    Ok(())
}

fn add_client_to_group(owned_client_groups: &mut HashMap<usize, ClientGroup>, group_id: &usize, connection: &Sender) -> Result<()> {
    if owned_client_groups.len() > *group_id {
        let destination_group = match owned_client_groups.get_mut(group_id) {
            Some(group) => group,
            None => {
                println!("Destination group doesn't exist");
                return Ok(());
            },
        };
        destination_group.clients.push(connection.connection_id());
    } else {
        let new_group = ClientGroup {
            group_id: *group_id,
            is_advertising: false,
            clients: vec![connection.connection_id()],
        };
        owned_client_groups.insert(*group_id, new_group);
    }
    return Ok(());
}

fn remove_client_from_group(owned_client_groups: &mut HashMap<usize, ClientGroup>, group_id: &usize, connection: &Sender) -> Result<()> {
    let group = match owned_client_groups.get_mut(&group_id) {
        Some(group) => group,
        None => {
            println!("Client belongs to nonexistent group");
            return Ok(());
        },
    };
    if group.clients.len() == 1 {
        owned_client_groups.remove(&group_id);
    } else {
        let client_index_in_group = match group.clients.iter().position(|client| *client == connection.connection_id()) {
            Some(index) => index,
            None => {
                println!("Couldn't find client in client group");
                return Ok(());
            },
        };
        group.clients.remove(client_index_in_group);
    };
    return Ok(());
}

fn new_service(shared_service_groups: &Mutex<[Vec<Sender>; 2]>, json: &MessageFormat, connection: &Sender) -> Result<ServiceType> {
    let service_type = match &json.strings {
        Some(string) => &string[0],
        None => {
            println!("Service didn't declare itself");
            return Ok(ServiceType::Unknown);
        },
    };
    let mut owned_service_groups = shared_service_groups.lock().unwrap();
    match service_type.as_str() {
        "MutualPlaylist" => {
            owned_service_groups[0].push(connection.clone());
            return Ok(ServiceType::MutualPlaylist);
        },
        &_ => {
            owned_service_groups[1].push(connection.clone());
            return Ok(ServiceType::Other)
        },
    };
}

fn join_group(shared_clients: &Mutex<HashMap<u32, Client>>, shared_client_groups: &Mutex<HashMap<usize, ClientGroup>>, json: &MessageFormat, connection: &Sender) -> Result<()> {
    let group_id = match json.id {
        Some(id) => id,
        None => {
            println!("Client didn't specify a group to join");
            return Ok(());
        },
    };
    let mut owned_clients = shared_clients.lock().unwrap();
    let ref mut owned_client_groups = shared_client_groups.lock().unwrap();
    if !owned_client_groups.contains_key(&group_id) { 
        println!("Client specified nonexistent group");
        return Err(Error {
            kind: ErrorKind::Internal,
            details: Cow::Owned(String::from("Client specified nonexistent group")),
        });
    }
    let current_client = match owned_clients.get_mut(&connection.connection_id()) {
        Some(client) => client,
        None => {
            println!("Client doesn't exist in map");
            return Ok(());
        },
    };
    remove_client_from_group(owned_client_groups, &current_client.group_id, connection).expect("Couldn't remove client from group");
    current_client.group_id = group_id;
    add_client_to_group(owned_client_groups, &current_client.group_id, connection).expect("Couldn't add client to group");
    let updated_group = owned_client_groups.get(&current_client.group_id);
    match updated_group {
        Some(group) => println!("Updated group: {:?}", group),
        None => {},
    }
    broadcast_client_groups(&owned_clients, &owned_client_groups).expect("Couldn't broadcast client groups");
    Ok(())
}

fn add_new_client(owned_clients: &mut HashMap<u32, Client>, owned_client_groups: &mut HashMap<usize, ClientGroup>, shared_client_group_count: &Mutex<usize>, token: &TokenInfo, connection: &Sender) -> Result<()> {
    let new_group_id = owned_client_groups.len();
    let new_client = Client {
        group_id: new_group_id,
        access_token: token.access_token.clone(),
        expires_at: Instant::now().checked_add(Duration::from_secs(token.expires_in as u64)).unwrap(),
        refresh_token: token.refresh_token.clone().unwrap(),
        connection: connection.clone(),
    };
    owned_clients.insert(connection.connection_id(), new_client);
    let mut owned_client_group_count = shared_client_group_count.lock().unwrap();
    add_client_to_group(owned_client_groups, &owned_client_group_count, connection).expect("Couldn't add client to group");
    *owned_client_group_count += 1;
    Ok(())
}

fn remove_client(owned_clients: &HashMap<u32, Client>, owned_client_groups: &mut HashMap<usize, ClientGroup>, connection: &Sender) -> Result<()> {
    let client = match owned_clients.get(&connection.connection_id()) {
        Some(client) => client,
        None => {
            println!("Client doesn't exist in map");
            return Ok(());
        },
    };
    remove_client_from_group(owned_client_groups, &client.group_id, connection).expect("Couldn't remove client from group");
    return Ok(())
}

fn broadcast_client_groups(owned_clients: &HashMap<u32, Client>, owned_client_groups: &HashMap<usize, ClientGroup>) -> Result<()> {
    let message = MessageFormat {
        message_type: MessageType::AdvertisingClientGroups,
        id: None,
        strings: None,
        groups: Some(owned_client_groups.values().map(|group| group.clone()).collect()),
    };
    let json = match serde_json::to_string(&message) {
        Ok(json) => json,
        Err(error) => {
            println!("Couldn't convert Message to string: {:?}", error);
            return Ok(());
        },
    };
    for client in owned_clients.values() {
        match client.connection.send(json.clone()) {
            Ok(()) => {},
            Err(error) => {
                println!("Couldn't send message to client: {:?}", error);
                return Ok(());
            },
        };
    };
    return Ok(());
}

fn make_mutual_playlist(shared_clients: &Mutex<HashMap<u32, Client>>, shared_client_groups: &Mutex<HashMap<usize, ClientGroup>>, shared_service_groups: &Mutex<[Vec<Sender>; 2]>, connection: &Sender) -> Result<()> {
    let owned_clients = shared_clients.lock().unwrap();
    let owned_client_groups = shared_client_groups.lock().unwrap();
    let current_client = match owned_clients.get(&connection.connection_id()) {
        Some(client) => client,
        None => {
            println!("Client doesn't exist in map");
            return Ok(());
        },
    };
    let current_group = match owned_client_groups.get(&current_client.group_id) {
        Some(group) => group,
        None => {
            println!("Client belongs to nonexistent group");
            return Ok(());
        },
    };
    if current_group.clients.len() < 2 {
        println!("Group that requested mutual playlist has less than two members");
        return Ok(());
    };
    let message = MessageFormat {
        message_type: MessageType::MakeMutualPlaylist,
        id: None,
        strings: Some(current_group.clients.iter().filter_map(|client| match owned_clients.get(client) {
            Some(client) => Some(client.access_token.clone()),
            None => None,
        }).collect()),
        groups: None,
    };
    let json = match serde_json::to_string(&message) {
        Ok(json) => json,
        Err(error) => {
            println!("Couldn't convert message to string: {:?}", error);
            return Ok(());
        },
    };
    let mut owned_service_groups = shared_service_groups.lock().unwrap();
    if owned_service_groups[0].len() == 0 {
        println!("There are no microservices currently prepared");
        return Ok(());
    };
    match owned_service_groups[0][0].send(json) {
        Ok(()) => {},
        Err(error) => {
            println!("Couldn't send message to client: {:?}", error);
            return Ok(());
        },
    };
    owned_service_groups[0].rotate_left(1);
    Ok(())
}

#[tokio::main]
async fn spotify_get_access_token(new_client: &str) -> Result<TokenInfo> {
    let oauth = SpotifyOAuth::default()
        .client_id("f092792439d74b7e9341f90719b98365")
        .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
        .redirect_uri("http://localhost:3000/home")
        .build();
    let token = match oauth.get_access_token(new_client).await {
        Some(token) => token,
        None => {
            println!("Couldn't get access token");
            return Err(Error {
                kind: ErrorKind::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid auth_code")),
                details: Cow::Owned(String::from("invalid auth_code")),
            });
        },
    };
    Ok(token)
}

#[tokio::main]
async fn spotify_pause(access_token: &str) -> Result<()> {
    let user = Spotify::default()
        .access_token(access_token)
        .build();
    match user.pause_playback(None).await {
        Ok(()) => {},
        Err(error) => {
            println!("Couldn't pause playback: {:?}", error);
            return Ok(());
        },
    };
    Ok(())
}

#[tokio::main]
async fn spotify_play(access_token: &str) -> Result<()> {
    let user = Spotify::default()
        .access_token(access_token)
        .build();
    match user.start_playback(None, None, None, None, None).await {
        Ok(()) => {},
        Err(error) => {
            println!("Couldn't pause playback: {:?}", error);
            return Ok(());
        },
    };
    Ok(())
}

#[tokio::main]
async fn refresh_token(refresh_token: &str) -> Result<TokenInfo> {
    let oauth = SpotifyOAuth::default()
        .client_id("f092792439d74b7e9341f90719b98365")
        .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
        .redirect_uri("http://localhost:3000/home")
        .build();
    match oauth.refresh_access_token(&refresh_token).await {
        Some(token) => return Ok(token),
        None => {
            println!("Empty response from refresh access token");
            return Err(Error {
                kind: ErrorKind::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to refresh access_token")),
                details: Cow::Owned(String::from("Failed to refresh access_token")),
            });
        },
    };
}

fn main() {
    let clients = Arc::new(Mutex::new(HashMap::new()));
    let client_groups = Arc::new(Mutex::new(HashMap::new()));
    let client_group_count = Arc::new(Mutex::new(0));
    let service_groups = Arc::new(Mutex::new([Vec::new(), Vec::new()]));
    let threads = Arc::new(ThreadPool::new(50));
    listen("192.168.1.69:8080", |connection| Server {connection: connection, connection_type: Arc::new(Mutex::new(ConnectionType::Unknown)), clients: Arc::clone(&clients), client_groups: Arc::clone(&client_groups), client_group_count: Arc::clone(&client_group_count), service_groups: Arc::clone(&service_groups), thread_pool: Arc::clone(&threads)}).unwrap();
}