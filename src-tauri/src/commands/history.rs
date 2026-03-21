use crate::models::track::RecentTracksPage;
use crate::state::AppState;

#[tauri::command]
pub async fn get_recent_tracks(
    state: tauri::State<'_, AppState>,
    page: u32,
    limit: u32,
) -> Result<RecentTracksPage, String> {
    let (username, client) = {
        let guard = state.0.lock().unwrap();
        (
            guard.settings.lastfm_username.clone(),
            guard.lastfm_client.clone(),
        )
    };

    if username.is_empty() {
        return Err("Last.fm ユーザー名が未設定です".to_string());
    }

    client.get_recent_tracks(&username, page, limit).await
}
