use crate::*;

pub(super) fn test_state(path: PathBuf) -> AppState {
    fn shared<T>(value: T) -> Arc<RwLock<T>> {
        Arc::new(RwLock::new(value))
    }
    AppState {
        library_upgrade: Arc::new(Mutex::new(Default::default())),
        library: shared(LibraryHandle { path, writer: None }),
        account_lock: Arc::new(Mutex::new(AccountLock::default())),
        theme: shared("light".into()),
        refresh_on_startup: shared(false),
        loader_timeout_secs: shared(10),
        feed_timeout_secs: shared(10),
        feed_max_body_bytes: shared(DEFAULT_FEED_MAX_BODY_BYTES),
        import_max_bytes: shared(DEFAULT_IMPORT_MAX_BYTES),
        html_reader_font_family: shared("system".into()),
        html_reader_font_size: shared(16.0),
        html_reader_line_height: shared(1.5),
        show_sidebar: shared(true),
        sidebar_sort: shared(sidebar_sort::SidebarSort::default()),
        embedding_service: Arc::new(Mutex::new(None)),
        embedding_query_model: Arc::new(Mutex::new(None)),
        embedding_startup_state: shared(EmbeddingStartupState::Unavailable),
        embeddings_read: Arc::new(Mutex::new(None)),
        allowed_domains: shared(Vec::new()),
        named_stores_supported: true,
        inline_pdf_reader: shared(false),
        load_remote_image: shared(true),
        html_reader_view: shared("summary".into()),
        network_proxy_port: 0,
        http_allowed: shared(false),
        connect_nonstandard_ports_allowed: shared(false),
        max_redirects: shared(0),
        referer_allowed: shared(false),
        gzip_allowed: shared(false),
        settings_baseline: shared(None),
    }
}
