use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

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

            // --- System Tray Icon (WhatsApp icon) ---
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

            // --- Notification Bridge JS ---
            let window = app.get_webview_window("main").unwrap();
            let js_bridge = r#"
                window._OriginalNotification = window.Notification;

                window.Notification = function(title, options) {
                    console.log('[WA-Notify] New notification:', title);

                    if (window.__TAURI__ && window.__TAURI__.notification) {
                        try {
                            window.__TAURI__.notification.sendNotification({
                                title: title || 'WhatsApp',
                                body: (options && options.body) || '',
                                icon: (options && options.icon) || undefined
                            });
                        } catch(e) {
                            console.error('[WA-Notify] Tauri notification error:', e);
                        }
                    }

                    return {
                        close: function() {},
                        addEventListener: function() {},
                        removeEventListener: function() {},
                        dispatchEvent: function() { return true; }
                    };
                };

                window.Notification.permission = 'granted';
                window.Notification.requestPermission = function() {
                    return Promise.resolve('granted');
                };

                console.log('[WA-Notify] Notification bridge installed');
            "#;
            let _ = window.eval(js_bridge);

            // --- Close to Tray (hide instead of quit) ---
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // Prevent default close, hide the window instead
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
