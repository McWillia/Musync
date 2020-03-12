use ws::{listen, Handler, Sender, Result, Message, Handshake, CloseCode, Error, ErrorKind};
use std::{
    sync::{Arc, RwLock, atomic::{AtomicUsize, Ordering}},
    time::{Instant, Duration},
    borrow::Cow,
};

use serde::{Serialize, Deserialize};
use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyOAuth, TokenInfo};
use dashmap::DashMap;

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
    sender: Sender,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClientGroup {
    group_id: usize,
    is_advertising: bool,
    clients: Vec<u32>,
}

#[derive(Clone)]
struct LocalState {
    sender: Sender,
    connection_type: Arc<RwLock<ConnectionType>>,
}

#[derive(Clone)]
struct SharedState {
    clients: Arc<DashMap<u32, Client>>,
    client_groups: Arc<DashMap<usize, ClientGroup>>,
    client_group_count: Arc<AtomicUsize>,
    service_groups: Arc<[RwLock<Vec<(u32, Sender)>>; 2]>,
}

struct Server {
    connection: Sender,
    connection_type: Arc<RwLock<ConnectionType>>,
    clients: Arc<DashMap<u32, Client>>,
    client_groups: Arc<DashMap<usize, ClientGroup>>,
    client_group_count: Arc<AtomicUsize>,
    service_groups: Arc<[RwLock<Vec<(u32, Sender)>>; 2]>,
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
        let shared_state = Arc::new(SharedState {
            clients: Arc::clone(&self.clients),
            client_groups: Arc::clone(&self.client_groups),
            client_group_count: Arc::clone(&self.client_group_count),
            service_groups: Arc::clone(&self.service_groups)
        });
        let local_state = Arc::new(LocalState {
            sender: self.connection.clone(),
            connection_type: Arc::clone(&self.connection_type)
        });
        tokio::spawn(handle_message(shared_state, local_state, Arc::new(message)));
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        let shared_state = SharedState {
            clients: Arc::clone(&self.clients),
            client_groups: Arc::clone(&self.client_groups),
            client_group_count: Arc::clone(&self.client_group_count),
            service_groups: Arc::clone(&self.service_groups),
        };
        let local_state = LocalState {
            sender: self.connection.clone(),
            connection_type: Arc::clone(&self.connection_type),
        };
        handle_close(&shared_state, &local_state, &code, &reason);
    }

    fn on_error(&mut self, err: Error) {
        println!("The server encountered an error: {:?}", err);
    }
}

async fn handle_message(shared_state: Arc<SharedState>, local_state: Arc<LocalState>, message: Arc<Message>) -> Result<()> {
    let text = match message.as_text() {
        Ok(text) => text,
        Err(error) => {
            println!("Message wasn't in string form: {:?}", error);
            return Ok(());
        },
    };
    println!("Message: {:?}", text);
    let json: MessageFormat = match serde_json::from_str(text){
        Ok(json) => json,
        Err(error) => {
            println!("Couldn't parse string to json: {:?}", error);
            return Ok(());
        },
    };
    match json.message_type {
        MessageType::NewClient => {
            new_client(&shared_state, &local_state, &json).await.expect("Could't make new client");
            *local_state.connection_type.write().unwrap() = ConnectionType::Client;
        },
        MessageType::MakeMutualPlaylist => {
            make_mutual_playlist(&shared_state, &local_state).await.expect("Couldn't make mutual playlist");
        },
        MessageType::JoinGroup => {
            join_group(&shared_state, &local_state, &json).expect("Couldn't join group");
        },
        MessageType::NewService => {
            match new_service(&shared_state, &local_state, &json) {
                Ok(service_type) => {
                    println!("New Service");
                    *local_state.connection_type.write().unwrap() = ConnectionType::Service(service_type);
                },
                Err(error) => {
                    println!("Couldn't add new service: {:?}", error);
                    return Ok(());
                },
            };
        },
        MessageType::Pause => {
            pause(&shared_state, &local_state).await.expect("Couldn't pause all users");
        },
        MessageType::Play => {
            play(&shared_state, &local_state).await.expect("Couldn't pause all users");
        },
        MessageType::AdvertisingClientGroups => {

        }
        _ => {
            println!("Unexpected Message Type: {:?}", json);
        },
    };
    Ok(())
}

async fn pause(shared_state: &SharedState, local_state: &LocalState) -> Result<()> {
    let current_client = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client,
        None => {
            println!("Couldn't find client in map");
            return Ok(());
        },
    };
    let current_group = match shared_state.client_groups.get(&current_client.group_id) {
        Some(group) => group,
        None => {
            println!("Client belongs to nonexistent group");
            return Ok(());
        },
    };
    for client_id in current_group.clients.iter() {
        check_refresh_client(&shared_state, &local_state).await.expect("Couldn't refresh access token");
            let access_token = match shared_state.clients.get(client_id) {
                Some(client) => client.access_token.to_owned(),
                None => {
                    println!("Client {} doesn't exist in map", client_id);
                    continue
                },
            };
        spotify_pause(&access_token).await.expect("Couldn't pause all server.clients");
    }
    Ok(())
}

async fn play(shared_state: &SharedState, local_state: &LocalState) -> Result<()> {
    let current_client = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client,
        None => {
            println!("Couldn't find client in map");
            return Ok(());
        },
    };
    let current_group = match shared_state.client_groups.get(&current_client.group_id) {
        Some(group) => group,
        None => {
            println!("Client belongs to nonexistent group");
            return Ok(());
        },
    };
    for client_id in current_group.clients.iter() {
        check_refresh_client(&shared_state, &local_state).await.expect("Couldn't refresh access token");
        let access_token = match shared_state.clients.get_mut(client_id) {
            Some(client) => client.access_token.to_owned(),
            None => {
                println!("Client {} doesn't exist in map", client_id);
                continue
            },
        };
        spotify_play(&access_token).await.expect("Couldn't pause all server.clients");
    }
    Ok(())
}

async fn check_refresh_client(shared_state: &SharedState, local_state: &LocalState) -> Result<()> {
    let expiry_time = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.expires_at.to_owned(),
        None => return Ok(()),
    };
    if Instant::now() > expiry_time {
        let mut client = shared_state.clients.get_mut(&local_state.sender.connection_id()).unwrap();
        match refresh_token(&client.refresh_token.to_owned()).await {
            Ok(token) => {
                client.access_token = token.access_token;
                match token.refresh_token {
                    Some(refresh_token) => client.refresh_token = refresh_token,
                    None => {},
                };
                client.expires_at = Instant::now().checked_add(Duration::from_secs(token.expires_in as u64)).unwrap();
            },
            Err(error) => {},
        }
    };
    Ok(())
}

async fn new_client(shared_state: &SharedState, local_state: &LocalState, json: &MessageFormat) -> Result<()> {
    let auth_code = match &json.strings {
        Some(code) => &code[0],
        None => {
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::from("Couldn't create new client"),
            });
        },
    };
    let token = spotify_get_access_token(&auth_code).await.expect("Couldn't get access token");
    add_new_client(&shared_state, &local_state, &token).expect("Couldn't add new client");
    broadcast_client_groups(&shared_state).expect("Couldn't broadcast client groups");
    Ok(())
}

fn add_client_to_group(shared_state: &SharedState, local_state: &LocalState, group_id: &usize) -> Result<()> {
    match shared_state.client_groups.get_mut(group_id) {
        Some(mut existing_group) => {
            existing_group.clients.push(local_state.sender.connection_id());
        },
        None => {
            shared_state.client_groups.insert(*group_id, ClientGroup {
                group_id: *group_id,
                is_advertising: false,
                clients: vec![local_state.sender.connection_id()],
            });
        },
    };
    return Ok(());
}

fn remove_client_from_group(shared_state: &SharedState, local_state: &LocalState, group_id: &usize) -> Result<()> {
    match shared_state.client_groups.remove_if(group_id, |_, group| {
        group.clients.len() == 1 && group.clients[0] == local_state.sender.connection_id()
    }) {
        Some(_result) => {
            //Group was removed
        },
        None => {
            let mut current_group = shared_state.client_groups.get_mut(group_id).unwrap();
            let client_index_in_group = match current_group.clients.iter().position(|client_id| *client_id == local_state.sender.connection_id()) {
                Some(index) => index,
                None => {
                    println!("Couldn't find client in client group");
                    return Ok(());
                },
            };
            current_group.clients.remove(client_index_in_group);
        }
    }
    return Ok(());
}

fn new_service(shared_state: &SharedState, local_state: &LocalState, json: &MessageFormat) -> Result<ServiceType> {
    let service_type = match &json.strings {
        Some(string) => &string[0],
        None => {
            println!("Service didn't declare itself");
            return Ok(ServiceType::Unknown);
        },
    };
    match service_type.as_str() {
        "MutualPlaylist" => {
            shared_state.service_groups[0].write().unwrap().push((local_state.sender.connection_id(), local_state.sender.to_owned()));
            return Ok(ServiceType::MutualPlaylist);
        },
        &_ => {
            shared_state.service_groups[1].write().unwrap().push((local_state.sender.connection_id(), local_state.sender.to_owned()));
            return Ok(ServiceType::Other)
        },
    };
}

fn join_group(shared_state: &SharedState, local_state: &LocalState, json: &MessageFormat) -> Result<()> {
    let group_id = match json.id {
        Some(id) => id,
        None => {
            println!("Client didn't specify a group to join");
            return Ok(());
        },
    };
    let old_group_id = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.group_id.to_owned(),
        None => {
            println!("Client doesn't exist in map");
            return Ok(());
        },
    };
    remove_client_from_group(&shared_state, &local_state, &old_group_id).expect("Couldn't remove client from group");
    add_client_to_group(&shared_state, &local_state, &group_id).expect("Couldn't add client to group");
    match shared_state.clients.get_mut(&local_state.sender.connection_id()) {
        Some(mut client) => {
            client.group_id = group_id;
        },
        None => {},
    };
    broadcast_client_groups(&shared_state).expect("Couldn't broadcast client groups");
    Ok(())
}

fn add_new_client(shared_state: &SharedState, local_state: &LocalState, token: &TokenInfo) -> Result<()> {
    let new_group_id = shared_state.client_group_count.fetch_add(1, Ordering::Relaxed);
    let new_client = Client {
        group_id: new_group_id,
        access_token: token.access_token.to_owned(),
        expires_at: Instant::now().checked_add(Duration::from_secs(token.expires_in as u64)).unwrap(),
        refresh_token: token.refresh_token.to_owned().unwrap(),
        sender: local_state.sender.to_owned(),
    };
    shared_state.clients.insert(local_state.sender.connection_id(), new_client);
    add_client_to_group(&shared_state, &local_state, &new_group_id).expect("Couldn't add client to group");
    Ok(())
}

fn remove_client(shared_state: &SharedState, local_state: &LocalState) -> Result<()> {
    let group_id = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.group_id.to_owned(),
        None => {
            println!("Client doesn't exist in map");
            return Ok(());
        },
    };
    remove_client_from_group(&shared_state, &local_state, &group_id).expect("Couldn't remove client from group");
    shared_state.clients.remove(&local_state.sender.connection_id());
    return Ok(())
}

fn broadcast_client_groups(shared_state: &SharedState) -> Result<()> {
    let message = MessageFormat {
        message_type: MessageType::AdvertisingClientGroups,
        id: None,
        strings: None,
        groups: Some(shared_state.client_groups.iter().map(|group| group.to_owned()).collect()),
    };
    let json = match serde_json::to_string(&message) {
        Ok(json) => json,
        Err(error) => {
            println!("Couldn't convert Message to string: {:?}", error);
            return Ok(());
        },
    };
    for client in shared_state.clients.iter() {
        match client.sender.send(Message::Text(json.to_owned())) {
            Ok(()) => {},
            Err(error) => {
                println!("Couldn't send message to client: {:?}", error);
                return Ok(());
            },
        };
    };
    return Ok(());
}

async fn make_mutual_playlist(shared_state: &SharedState, local_state: &LocalState) -> Result<()> {
    check_refresh_client(&shared_state, &local_state).await.expect("Couldn't refresh access token");
    let group_id = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.group_id.to_owned(),
        None => return Ok(()),
    };
    let current_group = match shared_state.client_groups.get(&group_id) {
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
        strings: Some(current_group.clients.iter().filter_map(|client| match shared_state.clients.get(client) {
            Some(client) => Some(client.access_token.to_owned()),
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
    let mut mutual_playlist_group = shared_state.service_groups[0].write().unwrap();
    if mutual_playlist_group.len() == 0 {
        println!("There are no microservices currently prepared");
        return Ok(());
    };
    match mutual_playlist_group[0].1.send(Message::Text(json)) {
        Ok(()) => {},
        Err(error) => {
            println!("Couldn't send message to client: {:?}", error);
            return Ok(());
        },
    };
    mutual_playlist_group.rotate_left(1);
    Ok(())
}

async fn spotify_get_access_token(new_client: &str) -> Result<TokenInfo> {
    let oauth = SpotifyOAuth::default()
        .client_id("f092792439d74b7e9341f90719b98365")
        .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
        .redirect_uri("http://localhost:3000/home")
        .build();
    let token = match oauth.get_access_token(new_client).await {
        Some(token) => token,
        None => {
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::from("Couldn't get access token"),
            });
        },
    };
    Ok(token)
}

async fn spotify_pause(access_token: &str) -> Result<()> {
    let user = Spotify::default()
        .access_token(access_token)
        .build();
    match user.pause_playback(None).await {
        Ok(()) => {},
        Err(error) => {
            println!("Couldn't pause playback: {:?}", error);
        },
    };
    Ok(())
}

async fn spotify_play(access_token: &str) -> Result<()> {
    let user = Spotify::default()
        .access_token(access_token)
        .build();
    match user.start_playback(None, None, None, None, None).await {
        Ok(()) => {},
        Err(error) => {
            println!("Couldn't pause playback: {:?}", error);
        },
    };
    Ok(())
}

async fn refresh_token(refresh_token: &str) -> Result<TokenInfo> {
    let oauth = SpotifyOAuth::default()
        .client_id("f092792439d74b7e9341f90719b98365")
        .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
        .redirect_uri("http://localhost:3000/home")
        .build();
    match oauth.refresh_access_token(&refresh_token).await {
        Some(token) => return Ok(token),
        None => {
            return Err(Error {
                kind: ErrorKind::Internal,
                details: Cow::from("Couldn't refresh access token"),
            });
        },
    };
}

fn handle_close(shared_state: &SharedState, local_state: &LocalState, code: &CloseCode, reason: &str) {
    println!("Connection {} closed: {}", local_state.sender.connection_id(), reason);
    match code {
        CloseCode::Normal => println!("The client closed the connection."),
        CloseCode::Away   => println!("The client left the site."),
        CloseCode::Abnormal => println!("Closing handshake failed! Unable to obtain closing status from client."),
        _ => println!("The client encountered an error: {}", reason),
    };
    match *local_state.connection_type.read().unwrap() {
        ConnectionType::Client => {
            if shared_state.clients.contains_key(&local_state.sender.connection_id()) {
                println!("Start", );
                remove_client(&shared_state, &local_state).expect("Couldn't remove client");
                println!("Middle", );
                broadcast_client_groups(&shared_state).expect("Couldn't broadcast client groups");
                println!("End", );
            };
        },
        ConnectionType::Service(service_type) => {
            let group_index = match service_type {
                ServiceType::MutualPlaylist => 0,
                ServiceType::Other => 1,
                ServiceType::Unknown => {
                    println!("Unknown service closed");
                    return;
                }
            };
            let service_index_in_group = match shared_state.service_groups[group_index].read().unwrap().iter().position(|service| service.0 == local_state.sender.connection_id()) {
                Some(index) => index.to_owned(),
                None =>  {
                    println!("Service local_state {} doesn't exist in service group list", local_state.sender.connection_id());
                    return;
                },
            };
            shared_state.service_groups[group_index].write().unwrap().remove(service_index_in_group);
        },
        ConnectionType::Unknown => {
            println!("Connection closed that hadn't done anything");
        }
    }
}

#[tokio::main]
async fn main() {
    let clients = Arc::new(DashMap::new());
    let client_groups = Arc::new(DashMap::new());
    let client_group_count = Arc::new(AtomicUsize::new(0));
    let service_groups = Arc::new([RwLock::new(Vec::new()), RwLock::new(Vec::new())]);
    listen("192.168.1.69:8080", |connection| Server {connection: connection, connection_type: Arc::new(RwLock::new(ConnectionType::Unknown)), clients: Arc::clone(&clients), client_groups: Arc::clone(&client_groups), client_group_count: Arc::clone(&client_group_count), service_groups: Arc::clone(&service_groups)}).unwrap();
}