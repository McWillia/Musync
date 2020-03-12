use serde::{Serialize, Deserialize};

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
    pub clients: Vec<u32>,
}