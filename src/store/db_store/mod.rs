mod author;
mod playlist;
mod track;
mod track_query_result;

use anyhow::Result;
use chrono::Utc;
use sea_orm::{
    prelude::Uuid, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    ModelTrait, QueryFilter, Set,
};
use serenity::{async_trait, futures::future::try_join_all};

use self::{
    playlist::{create_playlist, find_last_playlists},
    track::ToQueuedTrack,
};

use super::{
    guild_store::{CurrentTrack, Playlist, Store, TrackQueryResult},
    queued_track::QueuedTrack,
};

pub struct DbStore {
    conn: DatabaseConnection,
    playlist: Uuid,
    channel_id: u64,
}

impl DbStore {
    pub async fn create(conn: &DatabaseConnection, channel_id: &u64) -> Result<Self> {
        let playlist = create_playlist(conn, channel_id).await?;

        Ok(Self {
            conn: conn.clone(),
            playlist: playlist.id,
            channel_id: *channel_id,
        })
    }

    pub async fn create_with_current_playlist(
        conn: &DatabaseConnection,
        channel_id: &u64,
    ) -> Result<Option<Self>> {
        let playlist = find_last_playlists(conn, channel_id, None, 1).await?;
        let playlist = playlist.first();

        let playlist = match playlist {
            Some(playlist) => playlist,
            None => return Ok(None),
        };

        Ok(Some(Self {
            conn: conn.clone(),
            playlist: playlist.id,
            channel_id: *channel_id,
        }))
    }

    fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }
}

#[async_trait]
pub trait ToQueuedTracks {
    async fn to_queued_tracks(&self, conn: &DatabaseConnection) -> Result<Vec<QueuedTrack>>;
}

#[async_trait]
impl ToQueuedTracks for entity::playlist::Model {
    async fn to_queued_tracks(&self, conn: &DatabaseConnection) -> Result<Vec<QueuedTrack>> {
        let tracks = self.find_related(entity::track::Entity).all(conn).await?;

        let tracks_as_queued_tracks = tracks
            .into_iter()
            .map(|track| track.into_to_queued_track(conn));

        try_join_all(tracks_as_queued_tracks).await
    }
}

#[async_trait]
impl Store for DbStore {
    async fn queue(&self) -> Result<Vec<QueuedTrack>> {
        let playlist = self.get_playlist().await?;
        playlist.to_queued_tracks(self.conn()).await
    }

    async fn add_track_to_queue(&mut self, track: &QueuedTrack) -> Result<()> {
        let author = self
            .get_or_create_author(track.author() as i64, track.username())
            .await?;
        let track_query_result = self
            .get_or_create_track_query_result(track.title(), track.url(), track.duration())
            .await?;

        self.create_track(author.id, track_query_result.id).await?;

        Ok(())
    }

    async fn skip_track(&mut self, index: i32) -> Result<()> {
        if let Some(track) = self.find_track(index).await? {
            let mut model: entity::track::ActiveModel = track.into();
            model.skip = Set(true);
            model.updated_at = Set(Utc::now().into());
            model.insert(self.conn()).await?;
        }

        Ok(())
    }

    async fn find_track_query_result(&self, query: &str) -> Result<Option<TrackQueryResult>> {
        let res = entity::track_query::Entity::find()
            .filter(entity::track_query::Column::Query.eq(query))
            .one(self.conn())
            .await?;

        if let Some(track_query) = res {
            let res = track_query
                .find_related(entity::track_query_result::Entity)
                .one(self.conn())
                .await?;
            Ok(res.map(|item| item.into()))
        } else {
            Ok(None)
        }
    }

    async fn add_track_query_result(&self, query: &str, track: &QueuedTrack) -> Result<()> {
        let res = self
            .get_or_create_track_query_result(track.title(), track.url(), track.duration())
            .await?;

        let track_query = entity::track_query::ActiveModel {
            query: Set(query.into()),
            track_query_result: Set(res.id),
            ..Default::default()
        };

        track_query.insert(self.conn()).await?;

        Ok(())
    }

    async fn previous_queue(&self, max_playlists: u64) -> Result<Vec<Vec<QueuedTrack>>> {
        let playlists = self.find_last_playlists(max_playlists).await?;
        let to_queued_tracks = playlists
            .iter()
            .map(|playlist| playlist.to_queued_tracks(self.conn()));

        try_join_all(to_queued_tracks).await
    }

    async fn playlist(&self) -> Result<Playlist> {
        let playlist = self.get_playlist().await?;
        let is_last = playlist.last_track;
        let current_track = match playlist.current_track {
            Some(track) => {
                if is_last {
                    CurrentTrack::Last(track as usize)
                } else {
                    CurrentTrack::Index(track as usize)
                }
            }
            None => CurrentTrack::None,
        };

        Ok(Playlist { current_track })
    }

    async fn set_current_track(&mut self, current_track: CurrentTrack) -> Result<()> {
        let mut playlist = self.get_playlist().await?.into_active_model();
        playlist.updated_at = Set(Utc::now().into());
        match current_track {
            CurrentTrack::Last(index) => {
                playlist.last_track = Set(true);
                playlist.current_track = Set(Some(index as i32));
            }
            CurrentTrack::Index(index) => {
                playlist.last_track = Set(false);
                playlist.current_track = Set(Some(index as i32));
            }
            CurrentTrack::None => {
                playlist.last_track = Set(false);
                playlist.current_track = Set(None);
            }
        }
        playlist.update(self.conn()).await?;

        Ok(())
    }
}

impl From<entity::track_query_result::Model> for TrackQueryResult {
    fn from(model: entity::track_query_result::Model) -> Self {
        TrackQueryResult {
            title: model.title,
            url: model.url,
            duration: model.duration,
        }
    }
}
