use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy)]
pub enum ServiceType {
    MutualPlaylist,
    Other,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum MessageType {
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
pub struct MessageFormat {
    pub message_type: MessageType,
    pub id: Option<usize>,
    pub strings: Option<Vec<String>>,
    pub groups: Option<Vec<ClientGroup>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientGroup {
    pub group_id: usize,
    pub is_advertising: bool,
    pub clients: Vec<(u32, Option<String>)>,
}

pub struct MessageError {}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message Error")
    }
}
impl fmt::Debug for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Message Error")
    }
}

impl Error for MessageError {}

pub struct SharedStateError {}

impl fmt::Display for SharedStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Shared State Error")
    }
}
impl fmt::Debug for SharedStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Shared State Error")
    }
}

impl Error for SharedStateError {}

pub struct FunctionalityError {}

impl fmt::Display for FunctionalityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Functionality Error")
    }
}
impl fmt::Debug for FunctionalityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Functionality Error")
    }
}

impl Error for FunctionalityError {}