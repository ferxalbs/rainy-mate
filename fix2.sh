sed -i 's/tauri_app.run(|_app_handle, _event| {/tauri_app.run(|_app_handle, _event| {/g' src-tauri/src/lib.rs
# wait I didn't mean to do that. Let's just restore the `use tauri::Manager;`
sed -i '/use tauri::Builder;/i use tauri::Manager;' src-tauri/src/lib.rs
