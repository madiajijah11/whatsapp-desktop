use image::ImageEncoder;
use std::io::Cursor;
use std::process::Command;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

fn send_notification(title: &str, body: &str) {
    let title = title.to_string();
    let body = body.to_string();
    std::thread::spawn(move || {
        let _ = Command::new("notify-send")
            .args(["-a", "WhatsApp", "-t", "8000", "-i", "whatsapp", &title, &body])
            .status();
    });
}

#[tauri::command]
fn notify(title: String, body: String) {
    send_notification(&title, &body);
}

// ── Clipboard commands for WebKitGTK fallback ──

#[tauri::command]
fn clipboard_read_text() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.get_text().map_err(|e| e.to_string())
}

#[tauri::command]
fn clipboard_has_image() -> Result<bool, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    Ok(clipboard.get_image().is_ok())
}

#[tauri::command]
fn clipboard_read_image_base64() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let img = clipboard.get_image().map_err(|e| e.to_string())?;

    // Encode RGBA raw pixels as PNG in memory
    let mut png_buf = Vec::new();
    {
        let mut cursor = Cursor::new(&mut png_buf);
        let encoder = image::codecs::png::PngEncoder::new(&mut cursor);
        encoder
            .write_image(&img.bytes, img.width as u32, img.height as u32, image::ExtendedColorType::Rgba8)
            .map_err(|e| format!("PNG encode: {e}"))?;
    }

    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_buf);
    Ok(format!("data:image/png;base64,{b64}"))
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

    // ── Notification API override ──
    window.Notification = function(title, opts) {
        notifyRust(title || 'WhatsApp', (opts && opts.body) || '');
        return { close:function(){}, addEventListener:function(){}, removeEventListener:function(){}, dispatchEvent:function(){return true;} };
    };
    window.Notification.permission = 'granted';
    window.Notification.requestPermission = function() { return Promise.resolve('granted'); };

    // ── Watch title for unread count changes ──
    var lastTitle = '';
    setInterval(function() {
        var t = document.title;
        if (t && t !== lastTitle && t !== 'WhatsApp') {
            lastTitle = t;
            var match = t.match(/\((\d+)\)/);
            if (match) {
                notifyRust('WhatsApp', match[1] + ' unread messages');
            } else if (t.indexOf('WhatsApp') === -1 && t.length > 2) {
                notifyRust('WhatsApp', t);
            }
        }
    }, 2000);

    // ── Clipboard / Paste helper ──
    // WebKitGTK's Async Clipboard API doesn't support images, so we patch it.

    function tauriInvoke(cmd, args) {
        try {
            return window.__TAURI__.core.invoke(cmd, args || {});
        } catch(e) {
            return Promise.reject(e);
        }
    }

    function makeClipboardItem(blob) {
        var item = {};
        item[blob.type] = blob;
        return new ClipboardItem(item);
    }

    // 1) Patch navigator.clipboard.read() for the UI "Paste" button
    if (navigator.clipboard && !navigator.clipboard.__waPatched) {
        navigator.clipboard.__waPatched = true;
        var origRead = navigator.clipboard.read.bind(navigator.clipboard);

        navigator.clipboard.read = function() {
            return origRead().catch(function() {
                // Read image via Rust native clipboard fallback
                return tauriInvoke('clipboard_read_image_base64').then(function(imgB64) {
                    if (!imgB64) return tauriInvoke('clipboard_read_text').then(function(text) {
                        if (!text) throw new Error('Clipboard empty');
                        var b = new Blob([text], { type: 'text/plain' });
                        return [new ClipboardItem({ 'text/plain': b })];
                    });
                    return fetch(imgB64).then(function(r) { return r.blob(); }).then(function(blob) {
                        return [makeClipboardItem(blob)];
                    });
                });
            });
        };
    }

    // 2) Capture-phase paste handler for Ctrl+V image paste.
    //    Since WebKitGTK doesn't expose image clipboard data via paste events,
    //    we read the image natively (Rust/arboard) and dispatch a synthetic
    //    drop event with the image File — which WhatsApp Web handles natively.
    var pasteGuard = false;
    function getMessageInput() {
        return document.querySelector('div[role="textbox"][contenteditable="true"]')
            || document.querySelector('[contenteditable="true"]');
    }

    document.addEventListener('paste', function(e) {
        if (e.clipboardData && e.clipboardData.files && e.clipboardData.files.length > 0) return;
        if (pasteGuard) return;

        tauriInvoke('clipboard_has_image').then(function(hasImg) {
            if (!hasImg) return;

            e.preventDefault();
            e.stopPropagation();
            e.stopImmediatePropagation();

            tauriInvoke('clipboard_read_image_base64').then(function(imgB64) {
                if (!imgB64) return;

                fetch(imgB64).then(function(r) { return r.blob(); }).then(function(blob) {
                    var file = new File([blob], 'clipboard_image.png', { type: 'image/png' });

                    // Strategy A: dispatch a synthetic drop event (works in WhatsApp Web)
                    var dt = new DataTransfer();
                    dt.items.add(file);
                    var target = getMessageInput();
                    if (target) {
                        var dropEvent = new DragEvent('drop', {
                            dataTransfer: dt,
                            bubbles: true,
                            cancelable: true,
                            clientX: 0,
                            clientY: 0
                        });
                        target.dispatchEvent(dropEvent);
                    }

                    // Strategy B: write back to clipboard and re-trigger paste
                    // (fallback if drop didn't work)
                    pasteGuard = true;
                    navigator.clipboard.write([makeClipboardItem(blob)]).then(function() {
                        setTimeout(function() {
                            pasteGuard = false;
                            try { document.execCommand('paste'); } catch(ex) {}
                        }, 200);
                    }).catch(function() {
                        pasteGuard = false;
                    });
                }).catch(function() {});
            }).catch(function() {});
        }).catch(function() {});
    }, true); // capture phase
})();
"#;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // System Tray
            let open_item = MenuItem::with_id(app, "open", "Open WhatsApp", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
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

            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External("https://web.whatsapp.com".parse().unwrap()),
            )
            .title("WhatsApp")
            .inner_size(1200.0, 800.0)
            .min_inner_size(800.0, 600.0)
            .resizable(true)
            .center()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
            .initialization_script(WHATSAPP_BRIDGE)
            .build()?;

            // Close to Tray & Trim Memory
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.minimize();
                    #[cfg(target_os = "linux")]
                    unsafe {
                        extern "C" {
                            fn malloc_trim(pad: usize) -> i32;
                        }
                        malloc_trim(0);
                    }
                }
            });

            // Periodic memory trimming on Linux
            #[cfg(target_os = "linux")]
            std::thread::spawn(|| loop {
                std::thread::sleep(std::time::Duration::from_secs(300));
                unsafe {
                    extern "C" {
                        fn malloc_trim(pad: usize) -> i32;
                    }
                    malloc_trim(0);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![notify, clipboard_read_text, clipboard_has_image, clipboard_read_image_base64])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
