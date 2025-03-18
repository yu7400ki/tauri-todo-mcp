mod mcp;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let is_mcp = std::env::args_os().find(|a| a == "mcp").is_some();
            if is_mcp {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    crate::mcp::start_server(handle.clone()).await.unwrap();
                    handle.exit(0);
                });
            } else {
                tauri::webview::WebviewWindowBuilder::new(
                    app,
                    "main",
                    tauri::WebviewUrl::App("index.html".into()),
                )
                .title("tauri-todo-mcp")
                .build()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
