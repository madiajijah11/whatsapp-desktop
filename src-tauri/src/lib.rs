use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Listener, Manager,
};

const NOTIFICATION_BRIDGE: &str = r#"
(function() {
    if (window._waNotifyBridge) return;
    window._waNotifyBridge = true;

    // Override Notification API
    const OrigNotification = window.Notification;

    window.Notification = function(title, options) {
        console.log('[WA-Notify] Notification:', title);

        // Send via Tauri event to Rust side
        if (window.__TAURI__ && window.__TAURI__.event) {
            window.__TAURI__.event.emit('wa-notification', {
                title: title || 'WhatsApp',
                body: (options && options.body) || ''
            }).catch(function(e) {
                console.error('[WA-Notify] emit error:', e);
            });
        }

        // Return mock object so WhatsApp Web doesn't break
        return {
            close: function() {},
            addEventListener: function() {},
            removeEventListener: function() {},
            dispatchEvent: function() { return true; }
        };
    };

    // Grant permission
    Object.defineProperty(window.Notification, 'permission', {
        get: function() { return 'granted'; },
        configurable: true
    });
    window.Notification.requestPermission = function() {
        return Promise.resolve('granted');
    };

    console.log('[WA-Notify] Bridge installed');
})();
"#;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // --- System Tray Menu ---
            let open_item = MenuItem::with_id(app, "open", "Buka WhatsApp", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

            // --- System Tray Icon ---
            let _tray = TrayIconBuilder::new()
                .icon(tauri::include_image!("icons/whatsapp-tray.png"))
                .menu(&menu)
                .tooltip("WhatsApp Desktop")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.unminimize();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            let window = app.get_webview_window("main").unwrap();

            // --- Listen for notification events from webview ---
            let _app_handle = app.handle().clone();
            app.listen("wa-notification", move |event| {
                let payload = event.payload();
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(payload) {
                    let title = data["title"].as_str().unwrap_or("WhatsApp");
                    let body = data["body"].as_str().unwrap_or("");
                    println!("[WA-Notify] Sending: {} - {}", title, body);

                    // Use notify-rust directly for native notification
                    let mut notification = notify_rust::Notification::new();
                    notification.summary(title);
                    notification.body(body);
                    notification.icon("whatsapp");
                    notification.appname("WhatsApp Desktop");
                    let _ = notification.show();
                }
            });

            // --- Close to Tray ---
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            // --- Inject notification bridge ---
            // Inject now (for already-loaded page)
            let _ = window.eval(NOTIFICATION_BRIDGE);

            // Re-inject periodically to catch SPA navigations
            let window_timer = window.clone();
            std::thread::spawn(move || {
                for _ in 0..10 {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let _ = window_timer.eval(NOTIFICATION_BRIDGE);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
