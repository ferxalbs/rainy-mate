sed -i 's/tauri_app.run(|_app_handle, _event| {/tauri_app.run(|app_handle, event| {/g' src-tauri/src/lib.rs
sed -i 's/tauri_app.run(|app_handle, _event| {/tauri_app.run(|_app_handle, _event| {/g' src-tauri/src/lib.rs

# Let's fix the specific warnings cleanly instead of brute force.
sed -i 's/|app_handle, event|/|_app_handle, _event|/' src-tauri/src/lib.rs
