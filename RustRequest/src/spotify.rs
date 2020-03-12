use rspotify::oauth2::{SpotifyOAuth, TokenInfo};
use rspotify::client::Spotify;
use rspotify::senum::TimeRange;

pub async fn get_access_token(new_client: &str) -> Result<TokenInfo, String> {
    let oauth = SpotifyOAuth::default()
        .client_id("f092792439d74b7e9341f90719b98365")
        .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
        .redirect_uri("http://localhost:3000/home")
        .build();
    match oauth.get_access_token(new_client).await {
        Some(token) => Ok(token),
        None => {
            Err("Couldn't get access token".to_string())
        },
    }
}

pub async fn pause(access_token: &str) -> Result<(), String> {
    let user = Spotify::default()
        .access_token(access_token)
        .build();
    return match user.pause_playback(None).await {
        Ok(()) => Ok(()),
        Err(error) => {
            Err(format!("Couldn't pause playback: {:?}", error).to_string())
        },
    };
}

pub async fn play(access_token: &str) -> Result<(), String> {
    let user = Spotify::default()
        .access_token(access_token)
        .build();
    match user.start_playback(None, None, None, None, None).await {
        Ok(()) => Ok(()),
        Err(error) => {
            Err(format!("Couldn't pause playback: {:?}", error).to_string())
        },
    }
}

pub async fn refresh_token(refresh_token: &str) -> Result<TokenInfo, String> {
    let oauth = SpotifyOAuth::default()
        .client_id("f092792439d74b7e9341f90719b98365")
        .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
        .redirect_uri("http://localhost:3000/home")
        .build();
    match oauth.refresh_access_token(&refresh_token).await {
        Some(token) => Ok(token),
        None => {
            Err("Couldn't refresh token".to_string())
        },
    }
}

pub async fn get_user_top_tracks(spotify: &Spotify) -> Result<Vec<String>, String> {
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
            Err(error) => return Err(format!("Couldn't get user top tracks: {:?}", error)),
        };
    }
    return Ok(ids);
}

pub async fn create_playlist(spotify: &Spotify, common_tracks: Vec<String>) -> Result<(String, String), String> {
    let user = spotify
    .current_user()
    .await;
    let user = match user {
        Ok(user) => user,
        Err(error) => return Err(format!("Couldn't get current user: {:?}", error)),
    };
    let result = spotify
        .user_playlist_create(&user.id, "MutualPlaylist", Some(false), Some(String::from("MutualPlaylist")))
        .await;
        let playlist = match result {
        Ok(playlist) => playlist,
        Err(error) => return Err(format!("Error creating playlist: {:?}", error)),
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
            Some(error) => return Err(format!("Error adding tracks to playlist: {:?}", error)),
            None => {},
        };
    }
    let result = spotify
        .user_playlist_change_detail(&user.id, &playlist.id, None, None, None, Some(true))
        .await;
    match result {
        Ok(_) => return Ok((user.id, playlist.id)),
        Err(error) => return Err(format!("Error making playlist collaborative: {:?}", error)),
    };
}

pub async fn follow_playlist(spotify: &Spotify, owner_id: &str, playlist_id: &str) -> Result<(), String> {
    let result = spotify
        .user_playlist_follow_playlist(&owner_id, &playlist_id, None)
        .await;
        match result {
            Ok(_) => return Ok(()),
            Err(error) => return Err(format!("Error following playlist: {:?}", error)),
    }
}