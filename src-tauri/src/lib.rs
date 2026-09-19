use backend::models::{Account, Category, Transaction};
use backend::repository::Ledger;
use tauri::Manager;

#[tauri::command]
async fn get_accounts(ledger: tauri::State<'_, Ledger>) -> Result<Vec<Account>, String> {
  ledger.accounts().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn insert_account(
  ledger: tauri::State<'_, Ledger>,
  account: Account,
) -> Result<(), String> {
  ledger
    .insert_account(&account)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_categories(ledger: tauri::State<'_, Ledger>) -> Result<Vec<Category>, String> {
  ledger.categories().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn insert_category(
  ledger: tauri::State<'_, Ledger>,
  category: Category,
) -> Result<(), String> {
  ledger
    .insert_category(&category)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_transactions(ledger: tauri::State<'_, Ledger>) -> Result<Vec<Transaction>, String> {
  ledger.transactions().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn insert_transaction(
  ledger: tauri::State<'_, Ledger>,
  transaction: Transaction,
) -> Result<(), String> {
  ledger
    .insert_transaction(&transaction)
    .await
    .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let data_dir = app.path().app_data_dir()?;
      std::fs::create_dir_all(&data_dir)?;
      let db_url = format!("sqlite:{}", data_dir.join("ledger.db").display());
      let ledger = tauri::async_runtime::block_on(Ledger::connect(&db_url))?;
      app.manage(ledger);

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      get_accounts,
      insert_account,
      get_categories,
      insert_category,
      get_transactions,
      insert_transaction
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
