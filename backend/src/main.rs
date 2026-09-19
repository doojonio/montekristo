use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use backend::error::LedgerError;
use backend::models::{Account, Category, LedgerEntry, Transaction};
use backend::repository::Ledger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://ledger.db".to_string());
    let ledger = Ledger::connect(&database_url).await?;

    let app = Router::new()
        .route("/api/accounts", get(get_accounts).post(insert_account))
        .route("/api/accounts/{id}/ledger", get(get_account_ledger))
        .route(
            "/api/categories",
            get(get_categories).post(insert_category),
        )
        .route(
            "/api/transactions",
            get(get_transactions).post(insert_transaction),
        )
        // The Angular dev server runs on a separate origin.
        .layer(CorsLayer::permissive())
        .with_state(ledger);

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    eprintln!("listening on http://{bind_addr} (database: {database_url})");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_accounts(State(ledger): State<Ledger>) -> Result<Json<Vec<Account>>, ApiError> {
    Ok(Json(ledger.accounts().await?))
}

async fn insert_account(
    State(ledger): State<Ledger>,
    Json(account): Json<Account>,
) -> Result<(StatusCode, Json<Account>), ApiError> {
    ledger.insert_account(&account).await?;
    Ok((StatusCode::CREATED, Json(account)))
}

async fn get_account_ledger(
    State(ledger): State<Ledger>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<LedgerEntry>>, ApiError> {
    Ok(Json(ledger.account_ledger(id).await?))
}

async fn get_categories(State(ledger): State<Ledger>) -> Result<Json<Vec<Category>>, ApiError> {
    Ok(Json(ledger.categories().await?))
}

async fn insert_category(
    State(ledger): State<Ledger>,
    Json(category): Json<Category>,
) -> Result<(StatusCode, Json<Category>), ApiError> {
    ledger.insert_category(&category).await?;
    Ok((StatusCode::CREATED, Json(category)))
}

async fn get_transactions(State(ledger): State<Ledger>) -> Result<Json<Vec<Transaction>>, ApiError> {
    Ok(Json(ledger.transactions().await?))
}

async fn insert_transaction(
    State(ledger): State<Ledger>,
    Json(transaction): Json<Transaction>,
) -> Result<(StatusCode, Json<Transaction>), ApiError> {
    ledger.insert_transaction(&transaction).await?;
    Ok((StatusCode::CREATED, Json(transaction)))
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

/// HTTP adapter for [`LedgerError`]: renders failures as `{ "error": "..." }`
/// JSON bodies. Double-entry violations are client errors; everything else is
/// a server fault.
struct ApiError(LedgerError);

impl From<LedgerError> for ApiError {
    fn from(error: LedgerError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            LedgerError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            LedgerError::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(ErrorBody {
                error: self.0.to_string(),
            }),
        )
            .into_response()
    }
}
