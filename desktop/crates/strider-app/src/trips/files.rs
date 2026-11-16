use super::*;
impl AppView {
    pub(in crate::app) fn pick_trip_file(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose GPX, permit PDF/image, or trip JSON".into()),
        });
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(Ok(Some(paths))) => {
                    for p in paths {
                        this.trip_file(p, cx);
                    }
                }
                Ok(Ok(None)) => {}
                _ => {
                    this.error = Some("Could not open file picker.".into());
                    cx.notify();
                }
            });
        })
        .detach();
    }
    pub(in crate::app) fn trip_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let trip = self.trips.selected.clone();
        if let Some(d) = &self.trips.detail
            && d.access == "view"
        {
            return;
        }
        if !["gpx", "json", "pdf", "png", "jpg", "jpeg", "webp"].contains(&extension.as_str()) {
            self.error = Some("Choose a GPX, JSON, PDF or image file.".into());
            cx.notify();
            return;
        }
        // Read on the background executor; cap file size before allocating.
        let before: HashSet<_> = self.all_trips().iter().map(|t| t.sync_id.clone()).collect();
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("document")
            .to_owned();
        let task = cx.background_executor().spawn(async move {
            use std::io::Read;
            let file = std::fs::File::open(path)?;
            let mut bytes = Vec::new();
            file.take(15 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
            if bytes.len() > 15 * 1024 * 1024 {
                return Err(std::io::Error::other("File exceeds 15 MB."));
            }
            Ok(bytes)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                let cmd = (|| -> Result<Command, String> {
                    let bytes = result.map_err(|e| e.to_string())?;
                    if extension == "json" {
                        return Ok(Command::ImportTrip {
                            json: String::from_utf8(bytes)
                                .map_err(|_| "Trip JSON must be UTF-8.")?,
                        });
                    }
                    let trip = trip.ok_or("Open a trip first.")?;
                    if extension == "gpx" {
                        return Ok(Command::AttachGpx {
                            trip,
                            filename,
                            bytes,
                        });
                    }
                    let mime_type = match extension.as_str() {
                        "pdf" => "application/pdf",
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "webp" => "image/webp",
                        _ => return Err("Unsupported file.".into()),
                    }
                    .into();
                    Ok(Command::AttachPermit {
                        trip,
                        filename,
                        mime_type,
                        bytes,
                    })
                })();
                match cmd {
                    Ok(cmd) => {
                        let import = matches!(cmd, Command::ImportTrip { .. });
                        this.command(cmd, cx);
                        if import
                            && this.error.is_none()
                            && let Some(id) = this
                                .all_trips()
                                .iter()
                                .find(|t| !before.contains(&t.sync_id))
                                .map(|t| t.sync_id.clone())
                        {
                            this.open_trip(id, cx);
                        }
                    }
                    Err(e) => {
                        this.error = Some(e);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }
    pub(in crate::app) fn export_trip_gpx(&mut self, cx: &mut Context<Self>) {
        let Some(d) = &self.trips.detail else { return };
        let name = text(&d.fields, "name");
        let points: Vec<ExportPoint> = d.points.iter().map(ExportPoint::from).collect();
        let camps: Vec<_> = camps(d).into_iter().map(|(_, c)| c).collect();
        let xml = build_gpx(&name, &points, &camps);
        let filename = gpx_filename(&name);
        let receiver = cx.prompt_for_new_path(self.store.borrow().data_dir(), Some(&filename));
        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(path))) => {
                let task = cx
                    .background_executor()
                    .spawn(async move { std::fs::write(path, xml) });
                if let Err(e) = task.await {
                    let _ = this.update(cx, |this, cx| {
                        this.error = Some(e.to_string());
                        cx.notify();
                    });
                }
            }
            Ok(Ok(None)) => {}
            _ => {
                let _ = this.update(cx, |this, cx| {
                    this.error = Some("Could not open save panel.".into());
                    cx.notify();
                });
            }
        })
        .detach();
    }
    pub(super) fn open_permit(&mut self, cx: &mut Context<Self>) {
        let Some(p) = self.trips.detail.as_ref().and_then(|d| d.permit.clone()) else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let bytes = self
                .store
                .borrow()
                .blob_bytes(&p.sha256)
                .map_err(|e| e.to_string())?
                .ok_or("Not downloaded")?;
            let dir = self.store.borrow().data_dir().join("permit-viewer");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let filename = Path::new(&p.filename)
                .file_name()
                .ok_or("Invalid permit filename")?;
            let path = dir.join(filename);
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            std::process::Command::new("open")
                .arg(&path)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        if let Err(e) = result {
            self.error = Some(e);
            cx.notify();
        }
    }
    pub(in crate::app) fn configure_tiles(&mut self, cx: &mut Context<Self>) {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|p| p.join("Library/Caches/Strider"))
            .unwrap_or_else(|| self.store.borrow().data_dir().join("cache"));
        self.offline.backend.tiles_root = root.join("tiles");
        match strider_map::TileCache::new(
            root,
            strider_map::DEFAULT_DISK_CAP,
            32 * 1024 * 1024,
            4,
            Arc::new(strider_map::HttpFetcher),
        ) {
            Ok(cache) => {
                self.trips.map = cx.new(|cx| RouteMap::new(Some(Arc::new(cache)), cx));
                self.subscribe_routes(cx);
            }
            Err(e) => {
                self.error = Some(format!("Tile cache unavailable: {e}"));
            }
        }
    }
}
