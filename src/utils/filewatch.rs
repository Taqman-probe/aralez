use crate::utils::parceyaml::load_configuration;
use crate::utils::structs::Configuration;
use log::error;
#[cfg(unix)]
use notify::event::ModifyKind;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use pingora::prelude::sleep;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio::task;

fn is_watched_event(kind: &EventKind) -> bool {
    #[cfg(unix)]
    return matches!(kind, EventKind::Modify(ModifyKind::Data(_)) | EventKind::Create(..) | EventKind::Remove(..));
    #[cfg(windows)]
    return matches!(kind, EventKind::Modify(_) | EventKind::Create(..) | EventKind::Remove(..));
}

pub async fn start(fp: String, toreturn: Sender<Configuration>) {
    sleep(Duration::from_millis(50)).await; // For having nice logs :-)
    let file_path = fp.as_str();
    let parent_dir = Path::new(file_path).parent().unwrap();
    let (local_tx, mut local_rx) = tokio::sync::mpsc::channel::<notify::Result<Event>>(1);

    let _watcher_handle = task::spawn_blocking({
        let parent_dir = parent_dir.to_path_buf(); // Move directory path into the closure
        move || {
            let mut watcher = RecommendedWatcher::new(
                move |res| {
                    let _ = local_tx.blocking_send(res);
                },
                Config::default(),
            )
            .unwrap();
            watcher.watch(&parent_dir, RecursiveMode::Recursive).unwrap();
            let (_rtx, mut rrx) = tokio::sync::mpsc::channel::<bool>(1);
            let _ = rrx.blocking_recv();
        }
    });
    let mut start = Instant::now();

    while let Some(event) = local_rx.recv().await {
        match event {
            Ok(e) => {
                if is_watched_event(&e.kind) {
                    if start.elapsed() > Duration::from_secs(2) {
                        start = Instant::now();
                        let snd = load_configuration(file_path, "filepath").await.0;
                        if let Some(snd) = snd {
                            toreturn.send(snd).await.unwrap();
                        }
                    }
                }
            },
            Err(e) => error!("Watch error: {:?}", e),
        }
    }
}
