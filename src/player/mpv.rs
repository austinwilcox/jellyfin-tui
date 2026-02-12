use anyhow::{anyhow, Result};
use libmpv2::{events::Event as MpvEvent, events::EventContext, mpv_end_file_reason, Mpv};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Map libmpv2::Error (not Send/Sync due to Rc) to anyhow::Error
fn mpv_err(e: libmpv2::Error) -> anyhow::Error {
    anyhow!("mpv error: {e}")
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PlayerCommand {
    Play(String), // URL
    Pause,
    Resume,
    TogglePause,
    Stop,
    SeekForward(f64),
    SeekBackward(f64),
    SetVolume(i64),
    Quit,
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub playing: bool,
    pub paused: bool,
    pub position: f64,
    pub duration: f64,
    pub volume: i64,
    pub finished: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            playing: false,
            paused: false,
            position: 0.0,
            duration: 0.0,
            volume: 80,
            finished: false,
        }
    }
}

pub fn spawn_player_thread(
    cmd_rx: mpsc::Receiver<PlayerCommand>,
    state_tx: tokio::sync::mpsc::UnboundedSender<PlayerState>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        if let Err(e) = run_player(cmd_rx, state_tx) {
            eprintln!("Player thread error: {e}");
        }
    })
}

fn run_player(
    cmd_rx: mpsc::Receiver<PlayerCommand>,
    state_tx: tokio::sync::mpsc::UnboundedSender<PlayerState>,
) -> Result<()> {
    let mpv = Mpv::new().map_err(mpv_err)?;
    mpv.set_property("video", "no").map_err(mpv_err)?;
    mpv.set_property("cache", "yes").map_err(mpv_err)?;
    mpv.set_property("demuxer-max-bytes", "50MiB")
        .map_err(mpv_err)?;
    mpv.set_property("volume", 80i64).map_err(mpv_err)?;
    mpv.set_property("idle", "yes").map_err(mpv_err)?;

    let mut state = PlayerState::default();
    let mut ev_ctx = EventContext::new(mpv.ctx.clone());
    ev_ctx.enable_all_events().map_err(mpv_err)?;

    loop {
        // Process commands (non-blocking)
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                PlayerCommand::Play(url) => {
                    mpv.command("loadfile", &[&url, "replace"])
                        .map_err(mpv_err)?;
                    state.playing = true;
                    state.paused = false;
                    state.finished = false;
                    state.position = 0.0;
                }
                PlayerCommand::Pause => {
                    mpv.set_property("pause", true).map_err(mpv_err)?;
                    state.paused = true;
                }
                PlayerCommand::Resume => {
                    mpv.set_property("pause", false).map_err(mpv_err)?;
                    state.paused = false;
                }
                PlayerCommand::TogglePause => {
                    if state.playing {
                        let new_val = !state.paused;
                        mpv.set_property("pause", new_val).map_err(mpv_err)?;
                        state.paused = new_val;
                    }
                }
                PlayerCommand::Stop => {
                    mpv.command("stop", &[]).map_err(mpv_err)?;
                    state.playing = false;
                    state.paused = false;
                    state.position = 0.0;
                    state.duration = 0.0;
                }
                PlayerCommand::SeekForward(secs) => {
                    if state.playing {
                        mpv.command("seek", &[&secs.to_string(), "relative"])
                            .map_err(mpv_err)?;
                    }
                }
                PlayerCommand::SeekBackward(secs) => {
                    if state.playing {
                        mpv.command("seek", &[&(-secs).to_string(), "relative"])
                            .map_err(mpv_err)?;
                    }
                }
                PlayerCommand::SetVolume(vol) => {
                    let v = vol.clamp(0, 100);
                    mpv.set_property("volume", v).map_err(mpv_err)?;
                    state.volume = v;
                }
                PlayerCommand::Quit => {
                    return Ok(());
                }
            }
        }

        // Poll mpv events (drain all pending)
        while let Some(Ok(event)) = ev_ctx.wait_event(0.0) {
            match event {
                MpvEvent::EndFile(reason) => {
                    if reason == mpv_end_file_reason::Eof {
                        state.finished = true;
                        state.playing = false;
                    }
                    // Ignore Stop/Redirect — those fire when loadfile replaces
                    // the previous track and don't mean playback has stopped.
                }
                _ => {}
            }
        }

        // Update position/duration from mpv properties
        if let Ok(pos) = mpv.get_property::<f64>("time-pos") {
            state.position = pos;
        }
        if let Ok(dur) = mpv.get_property::<f64>("duration") {
            state.duration = dur;
        }

        // Send state update
        let _ = state_tx.send(state.clone());

        thread::sleep(Duration::from_millis(50));
    }
}
