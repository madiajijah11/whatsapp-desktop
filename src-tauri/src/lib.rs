use std::process::Command;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

fn send_notification(title: &str, body: &str) {
    let _ = Command::new("notify-send")
        .args(["-t", "8000", "-i", "whatsapp", title, body])
        .spawn();
}

#[tauri::command]
fn notify(title: String, body: String) {
    send_notification(&title, &body);
}

const WHATSAPP_BRIDGE: &str = r#"
(function() {
    if (window.__waBridgeInstalled) return;
    window.__waBridgeInstalled = true;

    var lastNotifyTime = 0;
    var lastNotifyBody = '';

    function notifyRust(title, body) {
        var now = Date.now();
        var key = title + ':' + body;
        if (key === lastNotifyBody && now - lastNotifyTime < 10000) return;
        if (now - lastNotifyTime < 3000) return;
        lastNotifyTime = now;
        lastNotifyBody = key;
        try {
            window.__TAURI__.core.invoke('notify', { title: title, body: body || '' });
        } catch(e) {}
    }

    // Override Notification API
    window.Notification = function(title, opts) {
        notifyRust(title || 'WhatsApp', (opts && opts.body) || '');
        return { close:function(){}, addEventListener:function(){}, removeEventListener:function(){}, dispatchEvent:function(){return true;} };
    };
    window.Notification.permission = 'granted';
    window.Notification.requestPermission = function() { return Promise.resolve('granted'); };

    // Watch title for unread count changes
    var lastTitle = '';
    setInterval(function() {
        var t = document.title;
        if (t && t !== lastTitle && t !== 'WhatsApp') {
            lastTitle = t;
            var match = t.match(/\((\d+)\)/);
            if (match) {
                notifyRust('WhatsApp', match[1] + ' pesan belum dibaca');
            } else if (t.indexOf('WhatsApp') === -1 && t.length > 2) {
                notifyRust('WhatsApp', t);
            }
        }
    }, 2000);
})();
"#;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // System Tray
            let open_item = MenuItem::with_id(app, "open", "Buka WhatsApp", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Keluar", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .icon(tauri::include_image!("icons/whatsapp-tray.png"))
                .menu(&menu)
                .tooltip("WhatsApp Desktop")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            let window = app.get_webview_window("main").unwrap();

            // Close to Tray
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.minimize();
                }
            });

            // Inject notification bridge into WhatsApp Web periodically
            let w = window.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(10));
                loop {
                    let _ = w.eval(WHATSAPP_BRIDGE);
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
            });

            // Ready notification
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_secs(2));
                send_notification("WhatsApp Desktop", "Siap digunakan!");
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![notify])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
