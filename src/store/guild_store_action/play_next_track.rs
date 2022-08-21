use anyhow::Result;
use serenity::async_trait;
use songbird::{Event, TrackEvent};
use songbird::{EventContext, EventHandler as SongbirdEventHandler};
use std::sync::Arc;
use tracing::info;

use super::{GuildStoreAction, GuildStoresActionHandler};
use crate::command_context::{voice::IntoInput, CommandContext};

impl GuildStoresActionHandler {
    pub async fn play_next_track(&mut self, ctx: Arc<CommandContext>) -> Result<()> {
        let store = self.get_or_create_store(&ctx).await?;
        if store.is_playing() {
            info!("Already playing a track, skipping.");
            return Ok(());
        }

        if let Some(next_track) = store.next_track_in_queue() {
            let handler_lock = ctx.songbird_call_lock().await?;

            let mut handler = handler_lock.lock().await;
            let next_input = next_track.to_input().await?;
            let handle = handler.play_only_source(next_input);
            let _ = handle.add_event(
                Event::Track(TrackEvent::End),
                SongEndNotifier::new(ctx.clone()),
            );

            ctx.send(format!("Now playing {}", next_track.title()))
                .await?;
        } else {
            let _ = ctx.send("End of playlist").await;
        }

        Ok(())
    }
}

#[async_trait]
impl SongbirdEventHandler for SongEndNotifier {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        let _ = self
            .ctx
            .send_action(GuildStoreAction::RemoveCurrentTrack(self.ctx.clone()))
            .await;
        let _ = self
            .ctx
            .send_action(GuildStoreAction::PlayNextTrack(self.ctx.clone()))
            .await;

        None
    }
}

struct SongEndNotifier {
    ctx: Arc<CommandContext>,
}

impl SongEndNotifier {
    fn new(ctx: Arc<CommandContext>) -> Self {
        Self { ctx }
    }
}