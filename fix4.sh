sed -i 's/|_app_handle, _event|/|app_handle, _event|/' src-tauri/src/lib.rs
sed -i 's/|app_handle, _event|/|app_handle, event|/' src-tauri/src/lib.rs
