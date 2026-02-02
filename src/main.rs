use actix_web::{
    App, HttpResponse, HttpServer, Responder,
    middleware::Logger,
    web::{self, Json, Path},
};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, SqlitePool, prelude::FromRow};

#[derive(Serialize, FromRow)]
struct Todo {
    id: i32,
    content: String,
}

#[derive(Deserialize)]
struct NewTodo {
    content: String,
}

async fn db() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePool::connect("sqlite:db.sqlite?mode=rwc")
        .await
        .unwrap();

    pool.execute(
        "CREATE TABLE IF NOT EXISTS todo ( 
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT
        )",
    )
    .await
    .expect("Failed to create table due to syntax error");
    pool
}

async fn get_todo_list(pool: web::Data<SqlitePool>) -> impl Responder {
    let todos: Vec<Todo> = sqlx::query_as("SELECT * FROM todo")
        .fetch_all(pool.get_ref())
        .await
        .unwrap();
    let todo_json = serde_json::to_string(&todos).unwrap();

    HttpResponse::Ok().body(todo_json)
}

async fn add_todo(todo: Json<NewTodo>, pool: web::Data<SqlitePool>) -> impl Responder {
    sqlx::query("INSERT INTO todo (content) VALUES (?1)")
        .bind(&todo.content)
        .execute(pool.get_ref())
        .await
        .unwrap();
    HttpResponse::Ok().body("OK")
}

async fn get_single_todo(id: Path<i32>, pool: web::Data<SqlitePool>) -> impl Responder {
    let id = id.into_inner();
    let row: Option<Todo> = sqlx::query_as("SELECT * FROM todo WHERE id = ?1")
        .bind(&id)
        .fetch_optional(pool.get_ref())
        .await
        .unwrap();

    match row {
        Some(todo) => HttpResponse::Ok().body(serde_json::to_string(&todo).unwrap()),
        None => HttpResponse::NotFound().body("Not Found"),
    }
}

async fn update_todo(
    id: Path<i32>,
    pool: web::Data<SqlitePool>,
    todo: Json<NewTodo>,
) -> impl Responder {
    let result = sqlx::query("UPDATE todo SET content = ?1 WHERE id = ?2")
        .bind(&todo.content)
        .bind(id.into_inner())
        .execute(pool.get_ref())
        .await
        .unwrap();

    if result.rows_affected() == 0 {
        HttpResponse::NotFound().body("Todo not found")
    } else {
        HttpResponse::Ok().body("Todo Updated")
    }
}
async fn delete_todo(id: Path<i32>, pool: web::Data<SqlitePool>) -> impl Responder {
    let id = id.into_inner();

    let result = sqlx::query("DELETE FROM todo WHERE id = ?1")
        .bind(&id)
        .execute(pool.get_ref())
        .await
        .unwrap();

    if result.rows_affected() == 0 {
        let msg = format!("No Todo with id: {} found!", id);
        HttpResponse::NotFound().body(msg)
    } else {
        HttpResponse::Ok().body("Todo Deleted")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let pool = db().await;

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .route("/todos", web::get().to(get_todo_list))
            .route("/todos", web::post().to(add_todo))
            .route("todos/{id}", web::get().to(get_single_todo))
            .route("todos/{id}", web::put().to(update_todo))
            .route("todos/{id}", web::delete().to(delete_todo))
    })
    .bind("0.0.0.0:8080")
    .unwrap()
    .run()
    .await
}
