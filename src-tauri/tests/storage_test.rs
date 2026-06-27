use ai_voice_input_lib::storage::{Storage, Transcript};

#[tokio::test]
async fn insert_and_list_transcripts() {
    let storage = Storage::in_memory().await.unwrap();
    storage.insert(&Transcript {
        id: None,
        raw_text: "嗯那个今天天气不错".into(),
        clean_text: "今天天气不错。".into(),
        duration_ms: 3000,
        created_at: chrono::Utc::now(),
        app_name: Some("notepad".into()),
    }).await.unwrap();

    let list = storage.list(10, 0).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].clean_text, "今天天气不错。");
}
