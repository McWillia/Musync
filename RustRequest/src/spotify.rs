use rspotify::oauth2::{SpotifyOAuth, TokenInfo};
use rspotify::client::Spotify;
use rspotify::senum::TimeRange;

use std::{
    borrow::Cow,
};

use ws::{Result, Error, ErrorKind};
use crate::communication::{
    FunctionalityError,
};

pub async fn get_access_token(new_client: &str) -> Result<TokenInfo> {
    let oauth = SpotifyOAuth::default()
        .client_id("f092792439d74b7e9341f90719b98365")
        .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
        .redirect_uri("http://localhost:3000/home")
        .build();
    return match oauth.get_access_token(new_client).await {
        Some(token) => Ok(token),
        None => Err(Error{
            kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
            details: Cow::Owned(format!("Couldn't get access token")),
        }),
    };
}

pub async fn pause(access_token: &str) -> Result<()> {
    let user = Spotify::default()
        .access_token(access_token)
        .build();
    return match user.pause_playback(None).await {
        Ok(()) => Ok(()),
        Err(error) => Err(Error{
            kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
            details: Cow::Owned(format!("Couldn't pause playback: {:?}", error)),
        }),
    };
}

pub async fn play(access_token: &str) -> Result<()> {
    let user = Spotify::default()
        .access_token(access_token)
        .build();
    return match user.start_playback(None, None, None, None, None).await {
        Ok(()) => Ok(()),
        Err(error) => Err(Error{
            kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
            details: Cow::Owned(format!("Couldn't start playback: {:?}", error)),
        }),
    };
}

pub async fn refresh_token(refresh_token: &str) -> Result<TokenInfo> {
    let oauth = SpotifyOAuth::default()
        .client_id("f092792439d74b7e9341f90719b98365")
        .client_secret("3b2f3bf79fc14c10967dca3dc97aacaf")
        .redirect_uri("http://localhost:3000/home")
        .build();
    return match oauth.refresh_access_token(&refresh_token).await {
        Some(token) => Ok(token),
        None => Err(Error{
            kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
            details: Cow::Owned(format!("Couldn't refresh token")),
        }),
    };
}

pub async fn get_user_top_tracks(spotify: &Spotify) -> Result<Vec<String>> {
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
            Err(error) => return Err(Error{
                kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
                details: Cow::Owned(format!("Couldn't get user top tracks: {:?}", error)),
            })
        };
    }
    return Ok(ids);
}

pub async fn create_playlist(spotify: &Spotify, common_tracks: Vec<String>) -> Result<(String, String)> {
    let user = spotify
    .current_user()
    .await;
    let user = match user {
        Ok(user) => user,
        Err(error) => return Err(Error{
            kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
            details: Cow::Owned(format!("Couldn't get current user: {:?}", error)),
        }),
    };
    let result = spotify
        .user_playlist_create(&user.id, "MutualPlaylist", Some(false), Some(String::from("MutualPlaylist")))
        .await;
    let playlist = match result {
        Ok(playlist) => playlist,
        Err(error) => return Err(Error{
            kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
            details: Cow::Owned(format!("Error creating playlist: {:?}", error)),
        }),
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
            Some(error) => return Err(Error{
                kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
                details: Cow::Owned(format!("Error adding tracks to playlist: {:?}", error)),
            }),
            None => {},
        };
    }
    let result = spotify
        .user_playlist_change_detail(&user.id, &playlist.id, None, None, None, Some(true))
        .await;
    return match result {
        Ok(_) => Ok((user.id, playlist.id)),
        Err(error) => Err(Error{
            kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
            details: Cow::Owned(format!("Error making playlist collaborative: {:?}", error)),
        }),
    };
}

pub async fn follow_playlist(spotify: &Spotify, owner_id: &str, playlist_id: &str) -> Result<()> {
    let result = spotify
        .user_playlist_follow_playlist(&owner_id, &playlist_id, None)
        .await;
    return match result {
        Ok(_) => Ok(()),
        Err(error) => Err(Error{
            kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
            details: Cow::Owned(format!("Error following playlist: {:?}", error)),
        }),
    };
}

pub async fn get_username(access_token: &str) -> Result<Option<String>> {
    let user = Spotify::default()
        .access_token(access_token)
        .build();
    return match user.current_user().await {
        Ok(user) => Ok(user.display_name),
        Err(error) => Err(Error{
            kind: ErrorKind::Custom(Box::new(FunctionalityError{})),
            details: Cow::Owned(format!("Couldn't get current user: {:?}", error)),
        }),
    };
}