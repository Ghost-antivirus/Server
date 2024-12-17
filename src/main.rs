use std::io::{self, Write};
use rand::{self, thread_rng, Rng};
use std::collections::HashMap;

struct User {
    password: String,
    token: Option<String>,
}

struct UserDatabase {
    users: HashMap<String, User>,
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
            .map(|(username, _)| username).cloned()
    }
}

fn list(users: &UserDatabase) -> Vec<String> {
    users.users.keys().cloned().collect() 
}

fn add(users: &mut UserDatabase, username: String, password: String) -> String {
    let user = User {
        password,
        token: None,
    };
    users.users.insert(username.clone(), user);
    format!("Пользователь '{}' добавлен.", username)
}

fn generate_token() -> String {
    let mut rng = thread_rng();
    let token: String = (0..16)
        .map(|_| rng.sample(rand::distributions::Alphanumeric))
        .map(char::from)
        .collect();
    token
}

fn auth(database: &mut UserDatabase, username: String, password: String) -> String {
    if let Some(user) = database.users.get_mut(&username) {
        if password == user.password {
            let token = generate_token();
            user.token = Some(token.clone());
            return format!("Аутентификация успешна. Токен для '{}': {}", username, token);
        }
    }
    "Ошибка аутентификации.".to_string()
}

fn logout(database: &mut UserDatabase, identifier: String) -> String {
    if let Some(username) = database.find_user_by_token(&identifier) {
        if let Some(user) = database.users.get_mut(&username) {
            user.token = None;
            return format!("Пользователь '{}' разлогинен.", username);
        }
    }

    if let Some(user) = database.users.get_mut(&identifier) {
        user.token = None;
        return format!("Пользователь '{}' разлогинен.", identifier);
    }

    "Пользователь или токен не найден.".to_string()
}

fn del(database: &mut UserDatabase, username: String) -> String {
    if database.users.remove(&username).is_some() {
        format!("Пользователь '{}' удален.", username)
    } else {
        format!("Пользователь '{}' не найден.", username)
    }
}

fn get_token(database: &UserDatabase, username: String) -> String {
    if let Some(user) = database.users.get(&username) {
        if let Some(token) = &user.token {
            return format!("Токен для '{}': {}", username, token);
        }
    }
    format!("Токен для '{}' не найден.", username)
}

fn main() {
    let mut users = UserDatabase::new();

    loop {
        println!("Запущена программа управления базы пользователей.\n");
        println!("Выберите режим работы:");
        println!("1. list - Выводит список пользователей. Можно вызвать 'list'.");
        println!("2. add - Добавляет пользователя. Можно вызвать 'add <username> <password>'.");
        println!("3. auth - Возвращает/генерирует токен (ключ сессии). Можно вызвать 'auth <username> <password>'.");
        println!("4. logout - Удаляет токен у соответствующего пользователя. Можно вызвать 'logout <username/token>'.");
        println!("5. del - Удаляет пользователя. Можно вызвать 'del <username>'.");
        println!("6. gettoken - Получает токен пользователя. Можно вызвать 'gettoken <username>'.");
        print!(">>> ");

        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).expect("Не удалось прочитать строку");
        let input = input.trim();
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            ["list"] => {
                let users_list = list(&users);
                if users_list.is_empty() {
                    println!("Нет пользователей.");
                } else {
                    println!("Список пользователей:");
                    for user in users_list {
                        println!("{}", user);
                    }
                }
            },
            ["add", username, password] => println!("{}", add(&mut users, username.to_string(), password.to_string())),
            ["auth", username, password] => println!("{}", auth(&mut users, username.to_string(), password.to_string())),
            ["logout", identifier] => println!("{}", logout(&mut users, identifier.to_string())),
            ["del", username] => println!("{}", del(&mut users, username.to_string())),
            ["gettoken", username] => println!("{}", get_token(&users, username.to_string())),
            ["exit"] => break,
            _ => println!("Неизвестная команда, попробуйте ещё раз."),
        }
    }
}