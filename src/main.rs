use postgres::{Client, NoTls};
use postgres::Error as PostgresError;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::env;
use log::info;
use log::warn;
use log::debug;
use log::error;
use log::log_enabled;
use log::Level;
use env_logger::Builder;

#[macro_use]
extern crate serde_derive;

// model
#[derive(Serialize, Deserialize)]
struct User {
    id: Option<i32>,
    name: String,
    email: String,
}

// const DB_URL: &str = env!("DB_URL");
const DB_URL: &str ="postgres://postgres:postgres@localhost:5432/postgres";
const SERVER_PORT: &str = "8080";
const RESPONSE_OK: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const RESPONSE_NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const RESPONSE_INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

fn main() {

    if let Err(e) = set_database() {
        println!("Error: {}", e);
        return;
    }
    Builder::new().init();
    // .parse_env(&env::var("MY_APP_LOG").unwrap_or_default())

    /*
        RUST_LOG="main::log::target=info"
     */
    log::info!("informational message");
    log::warn!("warning message");
    log::error!("this is an error {}", "message");

//     env_logger::init();
//
//     // debug!("Mary has a little lamb");
//     // warn!("{}", "The lamb was sure to go");
//     // warn!("{:#?}", "The lamb was sure to go");
//     // warn!("server started at port {} ..", SERVER_PORT);
//
//     if log_enabled!(Level::Error) {
//         error!("Error: {}", "Its fleece was white as snow");
//     }
//
//     if log_enabled!(Level::Info) {
//         info!("{}", "And every where that Mary went");
//         info!("{:?}", "And every where that Mary went");
//         info!("{}", "server started at port");
//     } else {
//         println!("log_enabled!(Level::Info) not enabled !");
//     }

    let listener = TcpListener::bind(format!("0.0.0.0:{}", SERVER_PORT)).unwrap();
    println!("server started at port {} ..", SERVER_PORT);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /users") => handle_post_request(r),
                r if r.starts_with("GET /user/") => handle_get_request(r),
                r if r.starts_with("GET /users") => handle_get_all_request(r),
                r if r.starts_with("PUT /users") => handle_put_request(r),
                r if r.starts_with("DELETE /users") => handle_delete_request(r),
                _ => (RESPONSE_NOT_FOUND.to_string(), "Not found".to_string()),
            };

            stream.write_all(format!("{}{}", status_line, content).as_bytes()).unwrap();
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

// controllers

fn handle_post_request(request: &str) -> (String, String) {
    match (get_user_request_body(&request), Client::connect(DB_URL, NoTls)) {
        (Ok(user), Ok(mut client)) => {
            client.execute("INSERT INTO users(name, email) VALUES ($1, $2)",
            &[&user.name, &user.email]
        ).unwrap();

        (RESPONSE_OK.to_string(), "User created".to_string())
        }
        _ => (RESPONSE_INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

fn handle_get_request(request: &str) -> (String, String) {
    println!("handle_get_request: {}", request);
    match (get_id(&request).parse::<i32>(), Client::connect(DB_URL, NoTls)) {

        (Ok(id), Ok(mut client)) =>
            match client.query_one("SELECT * FROM users WHERE id = $1", &[&id]) {
                Ok(row) => {
                    let user = User {
                        id: row.get(0),
                        name: row.get(1),
                        email: row.get(2),
                    };

                    (RESPONSE_OK.to_string(), serde_json::to_string(&user).unwrap())
                }
                _ => (RESPONSE_NOT_FOUND.to_string(), "User not found".to_string()),
            }

        (Err(e_db),Err(e_client)) => {
                println!("handle_get_request: {}, {}", e_db, e_client);
                (RESPONSE_INTERNAL_SERVER_ERROR.to_string(), "Internal error: handle_get_request".to_string())
        }

        _ => {
            println!("handle_get_request..");
            (RESPONSE_INTERNAL_SERVER_ERROR.to_string(), "Internal error: handle_get_request".to_string())
        }
    }
}

fn handle_get_all_request(_request: &str) -> (String, String) {
    println!("handle_get_all_request..");
    match Client::connect(DB_URL, NoTls) {
        Ok(mut client) => {
            let mut users = Vec::new();

            for row in client.query("SELECT id, name, email FROM users", &[]).unwrap() {
                users.push(User {
                    id: row.get(0),
                    name: row.get(1),
                    email: row.get(2),
                });
            }

            (RESPONSE_OK.to_string(), serde_json::to_string(&users).unwrap())
        }
        Err(e) => {
            println!("handle_get_all_request: {}", e);
            (RESPONSE_INTERNAL_SERVER_ERROR.to_string(), "Internal error: handle_get_all_request".to_string())
        }
        // _ => (RESPONSE_INTERNAL_SERVER_ERROR.to_string(), "Internal error".to_string()),
    }
}

fn handle_put_request(request: &str) -> (String, String) {
    match
        (
            get_id(&request).parse::<i32>(),
            get_user_request_body(&request),
            Client::connect(DB_URL, NoTls),
        )
    {
        (Ok(id), Ok(user), Ok(mut client)) => {
            client
                .execute(
                    "UPDATE users SET name = $1, email = $2 WHERE id = $3",
                    &[&user.name, &user.email, &id]
                )
                .unwrap();

            (RESPONSE_OK.to_string(), "User updated".to_string())
        }
        _ => (RESPONSE_INTERNAL_SERVER_ERROR.to_string(), "Internal error: handle_put_request".to_string()),
    }
}

fn handle_delete_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(DB_URL, NoTls)) {
        (Ok(id), Ok(mut client)) => {
            let rows_affected = client.execute("DELETE FROM users WHERE id = $1", &[&id]).unwrap();

            if rows_affected == 0 { //if rows affected is 0, user not found
                return (RESPONSE_NOT_FOUND.to_string(), "User not found".to_string());
            }

            (RESPONSE_OK.to_string(), "User deleted".to_string())
        }
        _ => (RESPONSE_INTERNAL_SERVER_ERROR.to_string(), "Internal error: handle_delete_request".to_string()),
    }
}

// Utility functions

fn set_database() -> Result<(), PostgresError> {
    let mut client = Client::connect(DB_URL, NoTls)?;
    client.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL
        )
    "
    )?;
    Ok(())
}

fn get_id(request: &str) -> &str {
    request.split("/")
        .nth(2)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

fn get_user_request_body(request: &str) -> Result<User, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}
