use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig,
};

#[derive(Debug, Clone)]
pub enum MediaEvent {
    Toggle,
    Next,
    Prev,
}

pub fn init_media_controls(
    event_tx: tokio::sync::mpsc::UnboundedSender<MediaEvent>,
) -> Option<MediaControls> {
    let config = PlatformConfig {
        dbus_name: "jellyfin_tui",
        display_name: "jellyfin-tui",
        hwnd: None,
    };

    let mut controls = match MediaControls::new(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to init media controls: {e:?}");
            return None;
        }
    };

    let tx = event_tx;
    if let Err(e) = controls.attach(move |event: MediaControlEvent| {
        let media_event = match event {
            MediaControlEvent::Toggle => Some(MediaEvent::Toggle),
            MediaControlEvent::Play => Some(MediaEvent::Toggle),
            MediaControlEvent::Pause => Some(MediaEvent::Toggle),
            MediaControlEvent::Next => Some(MediaEvent::Next),
            MediaControlEvent::Previous => Some(MediaEvent::Prev),
            _ => None,
        };
        if let Some(evt) = media_event {
            let _ = tx.send(evt);
        }
    }) {
        eprintln!("Failed to attach media controls: {e:?}");
        return None;
    }

    Some(controls)
}

pub fn update_metadata(controls: &mut MediaControls, title: &str, artist: &str, album: &str) {
    let _ = controls.set_metadata(MediaMetadata {
        title: Some(title),
        artist: Some(artist),
        album: Some(album),
        ..Default::default()
    });
}

pub fn update_playback(controls: &mut MediaControls, playing: bool, paused: bool) {
    let playback = if !playing {
        MediaPlayback::Stopped
    } else if paused {
        MediaPlayback::Paused { progress: None }
    } else {
        MediaPlayback::Playing { progress: None }
    };
    let _ = controls.set_playback(playback);
}

/// Service the macOS main run loop so that MPRemoteCommandCenter callbacks
/// (media key events from AirPods/headphones/keyboard) get delivered.
#[cfg(target_os = "macos")]
pub fn pump_event_loop() {
    type CFStringRef = *const std::ffi::c_void;
    extern "C" {
        static kCFRunLoopDefaultMode: CFStringRef;
        fn CFRunLoopRunInMode(
            mode: CFStringRef,
            seconds: f64,
            return_after_source_handled: u8,
        ) -> i32;
    }
    unsafe {
        CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.001, 0);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn pump_event_loop() {}
