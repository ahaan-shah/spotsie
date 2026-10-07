//! Diagnostic: which context shapes Spotify still resolves for a song
//! radio, run with the stored playback credential.
//!
//!   cargo run --example station_probe -- spotify:track:4uLU6hMCjMI75M1A2tKUQC

use librespot_core::{Session, SessionConfig, cache::Cache};
use librespot_protocol::autoplay_context_request::AutoplayContextRequest;

fn main() -> anyhow::Result<()> {
    fastframe_log::Logging::new("spotsie", env!("CARGO_PKG_VERSION"))
        .filter("warn")
        .init()?;
    let track = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "spotify:track:4uLU6hMCjMI75M1A2tKUQC".into());
    let id = track.rsplit(':').next().unwrap_or_default().to_string();

    let dirs = spotsie::paths::AppDirs::discover();
    let cache = Cache::new::<&std::path::Path>(None, None, None, None)?.with_memory_credentials();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let store = spotsie::credentials::Store::new(dirs);
        let loaded = store
            .lease(spotsie::credentials::Slot::Playback)
            .load()
            .await?;
        if let Some(warning) = loaded.warning {
            eprintln!("{warning}");
        }
        let Some(spotsie::credentials::Grant::Playback(credentials)) = loaded.grant else {
            anyhow::bail!("Enable playback in Spotsie first");
        };
        let session = Session::new(SessionConfig::default(), Some(cache));
        session.connect(credentials, false).await?;
        println!("connected as {}", session.username());

        let station = format!("spotify:station:track:{id}");
        for uri in [station.as_str(), track.as_str()] {
            print!("get_context({uri}) -> ");
            match session.spclient().get_context(uri).await {
                Ok(ctx) => println!(
                    "ok: uri={:?} pages={} first_page_tracks={}",
                    ctx.uri,
                    ctx.pages.len(),
                    ctx.pages.first().map(|p| p.tracks.len()).unwrap_or(0)
                ),
                Err(error) => println!("ERR: {error}"),
            }
        }
        for uri in [track.as_str(), station.as_str()] {
            let request = AutoplayContextRequest {
                context_uri: Some(uri.to_string()),
                ..Default::default()
            };
            print!("get_autoplay_context({uri}) -> ");
            match session.spclient().get_autoplay_context(&request).await {
                Ok(ctx) => println!(
                    "ok: uri={:?} pages={} first_page_tracks={}",
                    ctx.uri,
                    ctx.pages.len(),
                    ctx.pages.first().map(|p| p.tracks.len()).unwrap_or(0)
                ),
                Err(error) => println!("ERR: {error}"),
            }
        }
        // What Spotify's own "Go to radio" asks: the seed's radio playlist.
        for seed in std::env::args()
            .skip(1)
            .chain(std::iter::once(track.clone()))
        {
            let Ok(uri) = librespot_core::SpotifyUri::from_uri(&seed) else {
                continue;
            };
            print!("seed_to_playlist({seed}) -> ");
            match session.spclient().get_radio_for_track(&uri).await {
                Ok(body) => {
                    let text = String::from_utf8_lossy(&body);
                    println!("ok: {text}");
                    let playlist = serde_json::from_slice::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|json| json["mediaItems"][0]["uri"].as_str().map(str::to_string));
                    if let Some(playlist) = playlist {
                        print!("  get_context({playlist}) -> ");
                        match session.spclient().get_context(&playlist).await {
                            Ok(ctx) => println!(
                                "ok: pages={} tracks={}",
                                ctx.pages.len(),
                                ctx.pages.iter().map(|p| p.tracks.len()).sum::<usize>()
                            ),
                            Err(error) => println!("ERR: {error}"),
                        }
                    }
                }
                Err(error) => println!("ERR: {error}"),
            }
        }
        anyhow::Ok(())
    })?;
    Ok(())
}
