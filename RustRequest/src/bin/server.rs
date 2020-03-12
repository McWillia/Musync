use ws::{listen, Handler, Sender, Result as WSResult, Message, Handshake, CloseCode, Error as WSError, ErrorKind};
use std::{
    sync::{Arc, RwLock, atomic::{AtomicUsize, Ordering}},
    time::{Instant, Duration},
    borrow::Cow,
};

use rspotify::oauth2::TokenInfo;
use dashmap::DashMap;

use musink::communication::*;
use musink::spotify::*;

#[derive(Clone, Copy)]
pub enum ConnectionType {
    Client,
    Service(ServiceType),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Client {
    group_id: usize,
    access_token: String,
    expires_at: Instant,
    refresh_token: String,
    sender: Sender,
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

    fn on_open(&mut self, _: Handshake) -> WSResult<()> {
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

    fn on_message(&mut self, message: Message) -> WSResult<()> {
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

    fn on_error(&mut self, err: WSError) {
        println!("The server encountered an error: {:?}", err);
    }
}

async fn handle_message(shared_state: Arc<SharedState>, local_state: Arc<LocalState>, message: Arc<Message>) -> WSResult<()> {
    let text = match message.as_text() {
        Ok(text) => text,
        Err(error) => return Err(WSError {
            kind: ErrorKind::Internal,
            details: Cow::Owned(format!("Message wasn't in string format: {:?}", error))
        }),
    };
    println!("Message: {:?}", text);
    let json: MessageFormat = match serde_json::from_str(text){
        Ok(json) => json,
        Err(error) => return Err(WSError {
            kind: ErrorKind::Internal,
            details: Cow::Owned(format!("Couldn't parse json: {:?}", error))
        }),
    };
    match json.message_type {
        MessageType::NewClient => {
            match handle_new_client(&shared_state, &local_state, &json).await {
                Ok(()) => {
                    *local_state.connection_type.write().unwrap() = ConnectionType::Client;
                    return Ok(());
                },
                Err(error) => return Err(WSError {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(format!("Error handling play message: {:?}", error)),
                })
            }
        },
        MessageType::MakeMutualPlaylist => {
            match handle_make_mutual_playlist(&shared_state, &local_state).await {
                Ok(()) => return Ok(()),
                Err(error) => return Err(WSError {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(format!("Error handling make mutual playlist message: {:?}", error)),
                })
            }
        },
        MessageType::JoinGroup => {
            match handle_join_group(&shared_state, &local_state, &json) {
                Ok(()) => return Ok(()),
                Err(error) => return Err(WSError {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(format!("Error handling play message: {:?}", error)),
                })
            }
        },
        MessageType::NewService => {
            match handle_new_service(&shared_state, &local_state, &json) {
                Ok(service_type) => {
                    println!("New Service");
                    *local_state.connection_type.write().unwrap() = ConnectionType::Service(service_type);
                    return Ok(())
                },
                Err(error) => return Err(WSError {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(format!("Error handling new service message: {:?}", error)),
                }),
            };
        },
        MessageType::Pause => {
            match handle_pause(&shared_state, &local_state).await {
                Ok(()) => return Ok(()),
                Err(error) => return Err(WSError {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(format!("Error handling pause message: {:?}", error)),
                })
            }
        },
        MessageType::Play => {
            match handle_play(&shared_state, &local_state).await {
                Ok(()) => return Ok(()),
                Err(error) => return Err(WSError {
                    kind: ErrorKind::Internal,
                    details: Cow::Owned(format!("Error handling play message: {:?}", error)),
                })
            }
        },
        MessageType::AdvertisingClientGroups => {
            return Ok(());
        },
        _ => {
            return Err(WSError {
                kind: ErrorKind::Internal,
                details: Cow::Owned(format!("Unexpected Message Type: {:?}", json))
            });
        },
    };
}

async fn handle_pause(shared_state: &SharedState, local_state: &LocalState) -> Result<(), String> {
    let group_id = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.group_id.to_owned(),
        None => return Err(format!("Client {} doesn't exist in shared state", local_state.sender.connection_id())),
    };
    let current_group = match shared_state.client_groups.get(&group_id) {
        Some(group) => group,
        None => return Err(format!("Client group {} doesn't exist in shared state", group_id)),
    };
    for client_id in current_group.clients.iter() {
        check_refresh_client(&shared_state, &local_state).await.expect("Couldn't refresh access token");
        match shared_state.clients.get(client_id) {
            Some(client) => pause(&client.access_token.to_owned()).await?,
            None => {
                println!("Client {} doesn't exist in map", client_id);
                continue
            },
        };
    }
    Ok(())
}

async fn handle_play(shared_state: &SharedState, local_state: &LocalState) -> Result<(), String> {
    let group_id = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.group_id.to_owned(),
        None => return Err(format!("Client {} doesn't exist in shared state", local_state.sender.connection_id())),
    };
    let current_group = match shared_state.client_groups.get(&group_id) {
        Some(group) => group,
        None => return Err(format!("Client group {} doesn't exist in shared state", group_id)),
    };
    for client_id in current_group.clients.iter() {
        check_refresh_client(&shared_state, &local_state).await.expect("Couldn't refresh access token");
        match shared_state.clients.get(client_id) {
            Some(client) => play(&client.access_token.to_owned()).await?,
            None => {
                println!("Client {} doesn't exist in map", client_id);
                continue
            },
        };
    }
    Ok(())
}

async fn handle_new_client(shared_state: &SharedState, local_state: &LocalState, json: &MessageFormat) -> Result<(), String> {
    let auth_code = match &json.strings {
        Some(code) => &code[0],
        None => return Err(format!("Client {} didn't specify an auth code", local_state.sender.connection_id())),
    };
    let token = get_access_token(&auth_code).await?;
    add_new_client(&shared_state, &local_state, &token)?;
    broadcast_client_groups(&shared_state)?;
    Ok(())
}

fn handle_new_service(shared_state: &SharedState, local_state: &LocalState, json: &MessageFormat) -> Result<ServiceType, String> {
    let service_type = match &json.strings {
        Some(string) => &string[0],
        None => return Err("Service didn't specify its type".to_string()),
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

fn handle_join_group(shared_state: &SharedState, local_state: &LocalState, json: &MessageFormat) -> Result<(), String> {
    let group_id = match json.id {
        Some(id) => id,
        None => return Err("Client didn't specify a group to join".to_string()),
    };
    let old_group_id = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.group_id.to_owned(),
        None => return Err(format!("Client {} doesn't exist in the shared state", local_state.sender.connection_id())),
    };
    remove_client_from_group(&shared_state, &local_state, &old_group_id)?;
    add_client_to_group(&shared_state, &local_state, &group_id)?;
    match shared_state.clients.get_mut(&local_state.sender.connection_id()) {
        Some(mut client) => {
            client.group_id = group_id;
        },
        None => {},
    };
    broadcast_client_groups(&shared_state)?;
    Ok(())
}

fn add_client_to_group(shared_state: &SharedState, local_state: &LocalState, group_id: &usize) -> Result<(), String> {
    match shared_state.client_groups.get_mut(group_id) {
        Some(mut existing_group) => existing_group.clients.push(local_state.sender.connection_id()),
        None => {
            shared_state.client_groups.insert(*group_id, ClientGroup {
                group_id: *group_id,
                is_advertising: false,
                clients: vec![local_state.sender.connection_id()],
            });
        },
    };
    Ok(())
}

fn remove_client_from_group(shared_state: &SharedState, local_state: &LocalState, group_id: &usize) -> Result<(), String> {
    match shared_state.client_groups.remove_if(group_id, |_, group| {
        group.clients.len() == 1 && group.clients[0] == local_state.sender.connection_id()
    }) {
        Some(_result) => Ok(()),
        None => {
            let mut current_group = shared_state.client_groups.get_mut(group_id).unwrap();
            let client_index_in_group = match current_group.clients.iter().position(|client_id| *client_id == local_state.sender.connection_id()) {
                Some(index) => index,
                None => return Err(format!("Client {} isn't in shared state", local_state.sender.connection_id())),
            };
            current_group.clients.remove(client_index_in_group);
            Ok(())
        }
    }
}

fn add_new_client(shared_state: &SharedState, local_state: &LocalState, token: &TokenInfo) -> Result<(), String> {
    let new_group_id = shared_state.client_group_count.fetch_add(1, Ordering::Relaxed);
    let new_client = Client {
        group_id: new_group_id,
        access_token: token.access_token.to_owned(),
        expires_at: Instant::now().checked_add(Duration::from_secs(token.expires_in as u64)).unwrap(),
        refresh_token: token.refresh_token.to_owned().unwrap(),
        sender: local_state.sender.to_owned(),
    };
    shared_state.clients.insert(local_state.sender.connection_id(), new_client);
    add_client_to_group(&shared_state, &local_state, &new_group_id)?;
    Ok(())
}

fn remove_client(shared_state: &SharedState, local_state: &LocalState) -> Result<(), String> {
    let group_id = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.group_id.to_owned(),
        None => return Err(format!("Client {} doesn't exist in shared state", local_state.sender.connection_id())),
    };
    remove_client_from_group(&shared_state, &local_state, &group_id)?;
    shared_state.clients.remove(&local_state.sender.connection_id());
    return Ok(())
}

fn broadcast_client_groups(shared_state: &SharedState) -> Result<(), String> {
    let message = MessageFormat {
        message_type: MessageType::AdvertisingClientGroups,
        id: None,
        strings: None,
        groups: Some(shared_state.client_groups.iter().map(|group| group.to_owned()).collect()),
    };
    let json = match serde_json::to_string(&message) {
        Ok(json) => json,
        Err(error) => return Err(format!("Couldn't convert message to string: {:?}", error)),
    };
    for client in shared_state.clients.iter() {
        match client.sender.send(Message::Text(json.to_owned())) {
            Ok(()) => {},
            Err(error) => return Err(format!("Error sending message to client: {:?}", error)),
        };
    };
    return Ok(());
}

async fn handle_make_mutual_playlist(shared_state: &SharedState, local_state: &LocalState) -> Result<(), String> {
    check_refresh_client(&shared_state, &local_state).await.expect("Couldn't refresh access token");
    let group_id = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.group_id.to_owned(),
        None => return Err(format!("Client {} isn't in shared state", &local_state.sender.connection_id())),
    };
    let current_group = match shared_state.client_groups.get(&group_id) {
        Some(group) => group,
        None => return Err(format!("Client group {} doesn't exist ", group_id)),
    };
    if current_group.clients.len() < 2 {
        return Err("The current group has less than two members".to_string());
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
        Err(error) => return Err(format!("Couldn't convert message to string: {:?}", error)),
    };
    let mut mutual_playlist_group = shared_state.service_groups[0].write().unwrap();
    if mutual_playlist_group.len() == 0 {
        return Err("There are no microservices prepared to perform this task".to_string());
    };
    match mutual_playlist_group[0].1.send(Message::Text(json)) {
        Ok(()) => {},
        Err(error) => {
            return Err(format!("Error sending message to client: {:?}", error));
        },
    };
    mutual_playlist_group.rotate_left(1);
    Ok(())
}

async fn check_refresh_client(shared_state: &SharedState, local_state: &LocalState) -> Result<(), String> {
    let expiry_time = match shared_state.clients.get(&local_state.sender.connection_id()) {
        Some(client) => client.expires_at.to_owned(),
        None => return Err("Client isn't in shared state".to_string()),
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
                return Ok(());
            },
            Err(error) => return Err(format!("Error refreshing token: {:?}", error)),
        }
    }
    Ok(())
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
                remove_client(&shared_state, &local_state).expect("Couldn't remove client");
                broadcast_client_groups(&shared_state).expect("Couldn't broadcast client groups");
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