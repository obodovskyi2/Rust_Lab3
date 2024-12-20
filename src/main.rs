use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};

const TASKS_FILE: &str = "tasks.json";
const USERS_FILE: &str = "users.json";

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    id: u32,
    title: String,
    description: String,
    completed: bool,
    #[serde(with = "ts_seconds")]
    created_at: DateTime<Utc>,
    user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    username: String,
    password: String,
}

struct TodoApp {
    tasks: HashMap<u32, Task>,
    users: HashMap<String, User>,
    current_user: Option<String>,
    next_task_id: u32,
}

impl TodoApp {
    /// Create a new, empty `TodoApp`.
    fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            users: HashMap::new(),
            current_user: None,
            next_task_id: 1,
        }
    }

    /// Registers a new user. Returns an error if username already exists.
    fn register(&mut self, username: &str, password: &str) -> Result<(), &'static str> {
        if self.users.contains_key(username) {
            return Err("Username already exists");
        }

        self.users.insert(
            username.to_string(),
            User {
                username: username.to_string(),
                password: password.to_string(),
            },
        );
        self.save_users().map_err(|_| "Failed to save users")?;
        Ok(())
    }

    /// Logs in a user if the credentials are valid.
    fn login(&mut self, username: &str, password: &str) -> Result<(), &'static str> {
        match self.users.get(username) {
            Some(user) if user.password == password => {
                self.current_user = Some(username.to_string());
                Ok(())
            }
            _ => Err("Invalid username or password"),
        }
    }

    /// Adds a new task for the currently logged-in user.
    fn add_task(&mut self, title: &str, description: &str) -> Result<(), &'static str> {
        let user_id = self.current_user.clone().ok_or("Not logged in")?;

        let task = Task {
            id: self.next_task_id,
            title: title.to_string(),
            description: description.to_string(),
            completed: false,
            created_at: Utc::now(),
            user_id,
        };

        self.tasks.insert(self.next_task_id, task);
        self.next_task_id += 1;
        self.save_tasks().map_err(|_| "Failed to save tasks")?;
        Ok(())
    }

    /// Marks a task as completed if it belongs to the current user.
    fn complete_task(&mut self, task_id: u32) -> Result<(), &'static str> {
        let user_id = self.current_user.clone().ok_or("Not logged in")?;
        let task = self.tasks.get_mut(&task_id).ok_or("Task not found")?;
        if task.user_id != user_id {
            return Err("Not authorized to modify this task");
        }

        task.completed = true;
        self.save_tasks().map_err(|_| "Failed to save tasks")?;
        Ok(())
    }

    /// Edits the title and description of a user's task.
    fn edit_task(&mut self, task_id: u32, title: &str, description: &str) -> Result<(), &'static str> {
        let user_id = self.current_user.clone().ok_or("Not logged in")?;
        let task = self.tasks.get_mut(&task_id).ok_or("Task not found")?;
        if task.user_id != user_id {
            return Err("Not authorized to modify this task");
        }

        task.title = title.to_string();
        task.description = description.to_string();
        self.save_tasks().map_err(|_| "Failed to save tasks")?;
        Ok(())
    }

    /// Deletes a task if it belongs to the current user.
    fn delete_task(&mut self, task_id: u32) -> Result<(), &'static str> {
        let user_id = self.current_user.clone().ok_or("Not logged in")?;
        let task = self.tasks.get(&task_id).ok_or("Task not found")?;
        if task.user_id != user_id {
            return Err("Not authorized to delete this task");
        }

        self.tasks.remove(&task_id);
        self.save_tasks().map_err(|_| "Failed to save tasks")?;
        Ok(())
    }

    /// Lists all tasks belonging to the current user.
    fn list_tasks(&self) -> Result<Vec<&Task>, &'static str> {
        let user_id = self.current_user.as_ref().ok_or("Not logged in")?;
        Ok(self
            .tasks
            .values()
            .filter(|task| task.user_id == *user_id)
            .collect())
    }

    /// Saves all tasks to a JSON file.
    fn save_tasks(&self) -> io::Result<()> {
        let json = serde_json::to_string(&self.tasks)?;
        fs::write(TASKS_FILE, json)?;
        Ok(())
    }

    /// Loads tasks from the JSON file. If the file doesn't exist, it's ignored.
    fn load_tasks(&mut self) -> io::Result<()> {
        match fs::read_to_string(TASKS_FILE) {
            Ok(contents) => {
                self.tasks = serde_json::from_str(&contents)?;
                self.next_task_id = self.tasks.keys().max().map_or(1, |max| max + 1);
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // It's okay if no tasks file exists yet.
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Saves all users to a JSON file.
    fn save_users(&self) -> io::Result<()> {
        let json = serde_json::to_string(&self.users)?;
        fs::write(USERS_FILE, json)?;
        Ok(())
    }

    /// Loads users from a JSON file. If the file doesn't exist, it's ignored.
    fn load_users(&mut self) -> io::Result<()> {
        match fs::read_to_string(USERS_FILE) {
            Ok(contents) => {
                self.users = serde_json::from_str(&contents)?;
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Logs out the current user.
    fn logout(&mut self) {
        self.current_user = None;
    }

    fn is_logged_in(&self) -> bool {
        self.current_user.is_some()
    }
}

/// Helper function to print a prompt and read a trimmed line of input.
fn prompt_input(prompt: &str) -> Result<String, io::Error> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn main() -> io::Result<()> {
    let mut app = TodoApp::new();
    app.load_tasks()?;
    app.load_users()?;

    loop {
        if !app.is_logged_in() {
            // Display menu for non-logged in users
            println!("\nWelcome to Todo App!");
            println!("1. Login");
            println!("2. Register");
            println!("3. Exit");

            let choice = prompt_input("Select an option: ")?;
            match choice.as_str() {
                "1" => {
                    let username = prompt_input("Username: ")?;
                    let password = prompt_input("Password: ")?;
                    match app.login(&username, &password) {
                        Ok(_) => println!("Login successful!"),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                "2" => {
                    let username = prompt_input("Username: ")?;
                    let password = prompt_input("Password: ")?;
                    match app.register(&username, &password) {
                        Ok(_) => println!("Registration successful!"),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                "3" => {
                    println!("Goodbye!");
                    break;
                }
                _ => println!("Invalid choice"),
            }
        } else {
            // Display menu for logged in users
            println!("\nTodo App Menu:");
            println!("1. Add Task");
            println!("2. List Tasks");
            println!("3. Complete Task");
            println!("4. Edit Task");
            println!("5. Delete Task");
            println!("6. Logout");

            let choice = prompt_input("Select an option: ")?;
            match choice.as_str() {
                "1" => {
                    let title = prompt_input("Title: ")?;
                    let description = prompt_input("Description: ")?;
                    match app.add_task(&title, &description) {
                        Ok(_) => println!("Task added successfully!"),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                "2" => {
                    match app.list_tasks() {
                        Ok(tasks) => {
                            for task in tasks {
                                println!("\nID: {}", task.id);
                                println!("Title: {}", task.title);
                                println!("Description: {}", task.description);
                                println!("Status: {}", if task.completed { "Completed" } else { "Pending" });
                                println!("Created: {}", task.created_at);
                            }
                        }
                        Err(e) => println!("Error: {}", e),
                    }
                }
                "3" => {
                    let id_str = prompt_input("Task ID: ")?;
                    match id_str.parse::<u32>() {
                        Ok(task_id) => {
                            match app.complete_task(task_id) {
                                Ok(_) => println!("Task marked as completed!"),
                                Err(e) => println!("Error: {}", e),
                            }
                        }
                        Err(_) => println!("Invalid task ID"),
                    }
                }
                "4" => {
                    let id_str = prompt_input("Task ID: ")?;
                    let title = prompt_input("New Title: ")?;
                    let description = prompt_input("New Description: ")?;
                    match id_str.parse::<u32>() {
                        Ok(task_id) => {
                            match app.edit_task(task_id, &title, &description) {
                                Ok(_) => println!("Task updated successfully!"),
                                Err(e) => println!("Error: {}", e),
                            }
                        }
                        Err(_) => println!("Invalid task ID"),
                    }
                }
                "5" => {
                    let id_str = prompt_input("Task ID: ")?;
                    match id_str.parse::<u32>() {
                        Ok(task_id) => {
                            match app.delete_task(task_id) {
                                Ok(_) => println!("Task deleted successfully!"),
                                Err(e) => println!("Error: {}", e),
                            }
                        }
                        Err(_) => println!("Invalid task ID"),
                    }
                }
                "6" => {
                    app.logout();
                    println!("Logged out successfully!");
                }
                _ => println!("Invalid choice"),
            }
        }
    }

    Ok(())
}
