use rand::{self, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    collections::HashMap,
    thread
};
use tokio::{
    time::{Duration, timeout},
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream}
};
use std::string::String;

struct User {
    password: String,
    token: Option<String>,
}

struct UserDatabase {
    users: HashMap<String, User>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    command: String,
    data: Option<serde_json::Value>,
}

impl UserDatabase {
    fn new() -> Self {
        UserDatabase {
            users: HashMap::new(),
        }
    }

    fn find_user_by_token(&self, token: &str) -> Option<String> {
        self.users.iter()
            .find(|(_, user)| user.token.as_deref() == Some(token))
            .map(|(username, _)| username.clone())
    }
}

fn list(users: &UserDatabase) -> Option<Vec<String>> {
    if users.users.is_empty() {
        None
    } else {
        Some(users.users.keys().cloned().collect())
    }
}

fn add(users: &mut UserDatabase, username: String, password: String) -> Option<String> {
    if users.users.contains_key(&username) {
        if let Some(existing_user) = users.users.get(&username) {
            if existing_user.password == password {
                return None;
            }
        }
    }

    let user = User {
        password,
        token: None,
    };
    users.users.insert(username.clone(), user);
    Some(format!("Пользователь '{}' добавлен.", username))
}

fn generate_token() -> String {
    let mut rng = thread_rng();
    (0..16)
        .map(|_| rng.sample(rand::distributions::Alphanumeric))
        .map(char::from)
        .collect()
}

fn auth(database: &mut UserDatabase, username: String, password: String) -> Option<String> {
    if let Some(user) = database.users.get_mut(&username) {
        if password == user.password {
            let token = generate_token();
            user.token = Some(token.clone());
            return Some(token);
        }
    }
    None
}

fn logout(database: &mut UserDatabase, identifier: String) -> Option<String> {
    if let Some(username) = database.find_user_by_token(&identifier) {
        if let Some(user) = database.users.get_mut(&username) {
            user.token = None;
            return Some(format!("Пользователь '{}' разлогинен.", username));
        }
    }

    if let Some(user) = database.users.get_mut(&identifier) {
        user.token = None;
        return Some(format!("Пользователь '{}' разлогинен.", identifier));
    }

    None
}

fn del(database: &mut UserDatabase, username: String) -> Option<String> {
    if database.users.remove(&username).is_some() {
        Some(format!("Пользователь '{}' удален.", username))
    } else {
        None
    }
}

fn get_token(database: &UserDatabase, username: String) -> Option<String> {
    if let Some(user) = database.users.get(&username) {
        if let Some(token) = &user.token {
            return Some(token.to_string());
        }
    }
    None
}

fn auth_user(database: Arc<Mutex<UserDatabase>>, msg: Message) -> Message {
    let data = match msg.data {
        Some(serde_json::Value::Object(map)) => map,
        _ => return Message {
            command: "auth".to_string(),
            data: Some(serde_json::json!({"status": "err", "message": "Expected JSON object in 'data' field."})),
        },
    };

    let username = match data.get("username") {
        Some(serde_json::Value::String(username)) => username,
        _ => return Message {
            command: "auth".to_string(),
            data: Some(serde_json::json!({"status": "err", "message": "Field 'username' is missing or has the wrong type."})),
        },
    };

    let password = match data.get("password") {
        Some(serde_json::Value::String(password)) => password,
        _ => return Message {
            command: "auth".to_string(),
            data: Some(serde_json::json!({"status": "err", "message": "The 'password' field is missing or has the wrong type."})),
        },
    };

    let db = database.lock().unwrap();
    if let Some(token) = get_token(&db, username.to_string()) {
        return Message {
            command: "auth".to_string(),
            data: Some(serde_json::json!({"status": "ok", "message": token})),
        };
    }
    drop(db);

    let mut db = database.lock().unwrap();
    match auth(&mut db, username.to_string(), password.to_string()) {
        Some(token) => Message {
            command: "auth".to_string(),
            data: Some(serde_json::json!({"status": "ok", "message": token})),
        },
        None => Message {
            command: "auth".to_string(),
            data: Some(serde_json::json!({"status": "err", "message": "Invalid username or password."})),
        },
    }
}

// fn message_handler(database: Arc<Mutex<UserDatabase>>, msg: Message) {
//     let db = database.lock().unwrap();
//     match if let Some(token_value) = serde_json::to_string(&msg.data).get("token") {
//         if let Some(token_str) = token_value.as_str() {
//             db.find_user_by_token(token_str);
//         } else {
//             println!("Token не является строкой.");
//         }
//     } {
//         Some(_) => todo!(),
//         None => todo!(),
//     }
// }

fn database_manage(users: Arc<Mutex<UserDatabase>>) {
    println!("Запущена программа управления базы пользователей.\n");

    loop {
        println!("Выберите режим работы:");
        println!("1. list - Выводит список пользователей. Можно вызвать 'list'.");
        println!("2. add - Добавляет пользователя. Можно вызвать 'add <username> <password>'.");
        println!("3. auth - Возвращает/генерирует токен (ключ сессии). Можно вызвать 'auth <username> <password>'.");
        println!("4. logout - Удаляет токен у соответствующего пользователя. Можно вызвать 'logout <username/token>'.");
        println!("5. del - Удаляет пользователя. Можно вызвать 'del <username>'.");
        println!("6. gettoken - Получает токен пользователя. Можно вызвать 'gettoken <username>'.");
        println!("0. exit - для выхода.");
        print!(">>> ");

        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).expect("Не удалось прочитать строку");
        let input = input.trim();
        let parts: Vec<&str> = input.split_whitespace().collect();
        print!("\x1B[2J\x1B[1;1H");
        std::io::stdout().flush().unwrap();

        let mut db = users.lock().unwrap();
        match parts.as_slice() {
            ["list"] => {
                if let Some(users_list) = list(&db) {
                    println!("Список пользователей:");
                    for user in users_list {
                        println!("{}", user);
                    }
                } else {
                    println!("Нет пользователей.");
                }
            },
            ["add", username, password] => match add(&mut db, username.to_string(), password.to_string()) {
                Some(message) => println!("{}", message),
                None => println!("Ошибка: Пользователь с таким именем уже существует."),
            },
            ["auth", username, password] => match auth(&mut db, username.to_string(), password.to_string()) {
                Some(message) => println!("{}", message),
                None => println!("Ошибка аутентификации."),
            },
            ["logout", identifier] => match logout(&mut db, identifier.to_string()) {
                Some(message) => println!("{}", message),
                None => println!("Ошибка: Пользователь или токен не найден."),
            },
            ["del", username] => match del(&mut db, username.to_string()) {
                Some(message) => println!("{}", message),
                None => println!("Ошибка: Пользователь '{}' не найден.", username),
            },
            ["gettoken", username] => match get_token(&db, username.to_string()) {
                Some(message) => println!("{}", message),
                None => println!("Ошибка: Токен для '{}' не найден.", username),
            },
            ["exit"] => break,
            _ => println!("Неизвестная команда, попробуйте ещё раз."),
        }
    }
}

async fn handle_client(mut socket: TcpStream, addr: std::net::SocketAddr, database: Arc<Mutex<UserDatabase>>) {
    let mut buf = vec![0; 1024];
    println!("Клиент подключился: {}", addr);

    loop {
        let n = match socket.read(&mut buf).await {
            Ok(n) if n == 0 => {
                println!("Клиент {} отключился.", addr);
                return;
            }
            Ok(n) => n,
            Err(e) => {
                eprintln!("Ошибка при чтении от клиента {}: {:?}", addr, e);
                return;
            }
        };

        let msg: Message = match serde_json::from_slice(&buf[..n]) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Ошибка преобразования JSON от клиента {}: {:?}", addr, e);
                continue;
            }
        };

        let response = match msg.command.as_str() {
            "auth" => auth_user(database.clone(), msg),
            "message" => Message {
                command: "message".to_string(),
                data: msg.data.clone(),
            },
            _ => Message {
                command: "error".to_string(),
                data: Some(serde_json::json!({"message": "Unknown command"})),
            },
        };

        let response_json = match serde_json::to_vec(&response) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Ошибка преобразования JSON для клиента {}: {:?}", addr, e);
                continue;
            }
        };

        if let Err(e) = socket.write_all(&response_json).await {
            eprintln!("Ошибка при отправке данных клиенту {}: {:?}", addr, e);
            return;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let users = Arc::new(Mutex::new(UserDatabase::new()));
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Сервер запущен на 127.0.0.1");
    let users_manage = users.clone();
    let dat_man = thread::spawn(move || database_manage(users_manage));

    loop {
        if dat_man.is_finished() {
            return Ok(());
        }

        match timeout(Duration::from_secs(2), listener.accept()).await {
            Ok(Ok((socket, addr))) => {
                let users_clone = users.clone();
                tokio::spawn(handle_client(socket, addr, users_clone));
            }
            Ok(Err(e)) => {
                eprintln!("Ошибка при подключении: {:?}", e);
            }
            Err(_) => {
                print!("");
            }
        }
    } 
}
