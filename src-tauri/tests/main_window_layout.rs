//! Verify production main-window minimums against the native window manager.
//! Uses a blank webview; no account settings or user library are opened.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use objc2::{msg_send, runtime::AnyObject};
    use objc2_foundation::{NSRect, NSSize};
    use std::time::Duration;
    use tauri::{LogicalSize, WebviewUrl, WebviewWindowBuilder};

    tauri::Builder::default()
        .setup(|app| {
            let mut config = app.config().app.windows[0].clone();
            config.url = WebviewUrl::External("about:blank".parse()?);
            config.visible = true;
            let window = WebviewWindowBuilder::from_config(app, &config)?.build()?;
            // Cocoa's user-resize constraint is a frame minimum. Verify the
            // equivalent content size, excluding native window decorations.
            let native = unsafe { &*(window.ns_window()? as *const AnyObject) };
            let minimum: NSSize = unsafe { msg_send![native, minSize] };
            let frame: NSRect = unsafe { msg_send![native, frame] };
            let minimum_content: NSRect = unsafe {
                msg_send![native, contentRectForFrameRect: NSRect::new(frame.origin, minimum)]
            };
            assert_eq!(minimum_content.size.width, 320.0);
            assert_eq!(minimum_content.size.height, 256.0);
            let watchdog = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(15));
                eprintln!("FAIL: native main-window layout watchdog");
                watchdog.exit(1);
            });
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                for (requested, expected) in [
                    ((640.0, 512.0), (640.0, 512.0)),
                    ((320.0, 512.0), (320.0, 512.0)),
                    ((640.0, 256.0), (640.0, 256.0)),
                    ((320.0, 256.0), (320.0, 256.0)),
                ] {
                    window
                        .set_size(LogicalSize::new(requested.0, requested.1))
                        .unwrap();
                    let mut actual = (0.0, 0.0);
                    for _ in 0..50 {
                        std::thread::sleep(Duration::from_millis(50));
                        let size = window
                            .inner_size()
                            .unwrap()
                            .to_logical::<f64>(window.scale_factor().unwrap());
                        actual = (size.width, size.height);
                        if actual == expected {
                            break;
                        }
                    }
                    if actual != expected {
                        eprintln!(
                            "FAIL: requested {requested:?}, expected {expected:?}, got {actual:?}"
                        );
                        handle.exit(1);
                        return;
                    }
                }
                println!("PASS: native main-window minimum is 320 × 256 logical pixels");
                handle.exit(0);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("native main-window layout fixture");
}
