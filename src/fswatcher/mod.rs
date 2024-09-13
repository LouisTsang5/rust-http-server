use notify::{EventKind, RecursiveMode, Watcher};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    path::PathBuf,
    sync::Arc,
};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{error, filecache::FileCache, log_ctx, requestmap::RequestMap, trace, BUFF_INIT_SIZE};
log_ctx!("FSWatcher");

#[derive(Debug)]
pub enum WatcherError {
    InitError,
    EventError(notify::Error),
    ChannelClosed,
}

impl Error for WatcherError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl Display for WatcherError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            WatcherError::InitError => write!(f, "Error initializing watcher"),
            WatcherError::EventError(err) => write!(f, "Error receiving event: {}", err),
            WatcherError::ChannelClosed => write!(f, "Event channel closed"),
        }
    }
}

pub fn setup_fs_watcher(
    ctx: Arc<(FileCache, Option<RequestMap>, PathBuf)>,
) -> notify::Result<JoinHandle<Result<(), WatcherError>>> {
    // create watcher and event channel
    let (tx, mut rx) = mpsc::channel(BUFF_INIT_SIZE);
    let mut watcher = notify::recommended_watcher(move |res| tx.blocking_send(res).unwrap())?;

    // spawn watcher task
    let t: JoinHandle<Result<(), WatcherError>> = tokio::spawn(async move {
        let (file_cache, _, res_root) = ctx.as_ref();

        // watch res folder
        if let Err(err) = watcher.watch(res_root, RecursiveMode::Recursive) {
            error!("Error watching directory: {}", err);
            return Err(WatcherError::InitError);
        }

        // event loop
        while let Some(e) = rx.recv().await {
            let event = match e {
                Ok(event) => {
                    trace!("Folder event: {:?}", event);
                    match event.kind {
                        EventKind::Modify(_) => Some(event),
                        EventKind::Remove(_) => Some(event),
                        _ => None,
                    }
                }
                Err(err) => return Err(WatcherError::EventError(err)),
            };

            // remove file from cache
            if let Some(event) = event {
                for path in event.paths {
                    let removed = file_cache.remove(&path).await;
                    if let Some(_) = removed {
                        trace!("Removed {} from file cache", path.display());
                    }
                }
            }
        }

        // return error if channel closed
        Err(WatcherError::ChannelClosed)
    });

    // return watcher task
    Ok(t)
}
