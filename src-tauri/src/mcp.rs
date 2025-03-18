use std::{future::Future, pin::Pin};

use mcp_core::{
    handler::{PromptError, ResourceError},
    prompt::Prompt,
    protocol::ServerCapabilities,
    Content, Tool, ToolError,
};
use mcp_server::{
    router::{CapabilitiesBuilder, RouterService},
    ByteTransport, Server,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tokio::io::{stdin, stdout};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Todo {
    id: u64,
    text: String,
    done: bool,
}

#[derive(Clone)]
pub struct TodoRouter {
    app: AppHandle,
}

const STORE_PATH: &str = "store.json";
const TODOS_KEY: &str = "todos";

impl TodoRouter {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    fn get_todos(&self) -> Result<Vec<Todo>, ToolError> {
        let store = self
            .app
            .store(STORE_PATH)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        store
            .reload()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        let todos = store
            .get(TODOS_KEY)
            .and_then(|value| serde_json::from_value::<Vec<Todo>>(value).ok())
            .unwrap_or_else(|| vec![]);
        Ok(todos)
    }

    fn add_todo(&self, text: String) -> Result<Todo, ToolError> {
        let store = self
            .app
            .store(STORE_PATH)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        let mut todos = self.get_todos()?;
        let id = chrono::Utc::now().timestamp_millis() as u64;
        let todo = Todo {
            id,
            text,
            done: false,
        };
        todos.push(todo.clone());
        store.set(
            TODOS_KEY,
            serde_json::to_value(todos).map_err(|e| ToolError::ExecutionError(e.to_string()))?,
        );
        store
            .save()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        Ok(todo)
    }

    fn remove_todo(&self, id: u64) -> Result<(), ToolError> {
        let store = self
            .app
            .store(STORE_PATH)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        let mut todos = self.get_todos()?;
        todos.retain(|todo| todo.id != id);
        store.set(
            TODOS_KEY,
            serde_json::to_value(todos).map_err(|e| ToolError::ExecutionError(e.to_string()))?,
        );
        store
            .save()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        Ok(())
    }

    fn update_todo(&self, todo: Todo) -> Result<(), ToolError> {
        let store = self
            .app
            .store(STORE_PATH)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        let mut todos = self.get_todos()?;
        if let Some(index) = todos.iter().position(|t| t.id == todo.id) {
            todos[index] = todo;
            store.set(
                TODOS_KEY,
                serde_json::to_value(todos)
                    .map_err(|e| ToolError::ExecutionError(e.to_string()))?,
            );
        }
        store
            .save()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;
        Ok(())
    }
}

impl mcp_server::Router for TodoRouter {
    fn name(&self) -> String {
        "todo".to_string()
    }

    fn instructions(&self) -> String {
        "This server allows you to manage todos with persistent storage. You can retrieve the current list of todos using `get_todos`, add a new todo with `add_todo`, remove a specific todo by its ID using `remove_todo`, and update an existing todo with `update_todo`.".to_string()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .with_prompts(false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "get_todos".to_string(),
                "Get Todos".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            Tool::new(
                "add_todo".to_string(),
                "Add Todo".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string"
                        }
                    },
                    "required": ["text"]
                }),
            ),
            Tool::new(
                "remove_todo".to_string(),
                "Remove Todo".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "integer"
                        }
                    },
                    "required": ["id"]
                }),
            ),
            Tool::new(
                "update_todo".to_string(),
                "Update Todo".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "integer"
                        },
                        "text": {
                            "type": "string"
                        },
                        "done": {
                            "type": "boolean"
                        }
                    },
                    "required": ["id", "text", "done"]
                }),
            ),
        ]
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<Vec<Content>, ToolError>> + Send + 'static>>
    {
        let this = self.clone();
        let tool_name = tool_name.to_string();

        Box::pin(async move {
            match tool_name.as_str() {
                "get_todos" => {
                    let todos = this.get_todos()?;
                    Ok(vec![Content::text(serde_json::to_string(&todos).unwrap())])
                }
                "add_todo" => {
                    let text = arguments["text"]
                        .as_str()
                        .ok_or_else(|| ToolError::InvalidParameters("text".to_string()))?
                        .to_string();
                    let todo = this.add_todo(text)?;
                    Ok(vec![Content::text(serde_json::to_string(&todo).unwrap())])
                }
                "remove_todo" => {
                    let id = arguments["id"]
                        .as_u64()
                        .ok_or_else(|| ToolError::InvalidParameters("id".to_string()))?;
                    this.remove_todo(id)?;
                    Ok(vec![Content::text("".to_string())])
                }
                "update_todo" => {
                    let id = arguments["id"]
                        .as_u64()
                        .ok_or_else(|| ToolError::InvalidParameters("id".to_string()))?;
                    let text = arguments["text"]
                        .as_str()
                        .ok_or_else(|| ToolError::InvalidParameters("text".to_string()))?
                        .to_string();
                    let done = arguments["done"]
                        .as_bool()
                        .ok_or_else(|| ToolError::InvalidParameters("done".to_string()))?;
                    let todo = Todo { id, text, done };
                    this.update_todo(todo)?;
                    Ok(vec![Content::text("".to_string())])
                }
                _ => Err(ToolError::NotFound(tool_name)),
            }
        })
    }

    fn list_resources(&self) -> Vec<mcp_core::resource::Resource> {
        vec![]
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        Box::pin(async move { Err(ResourceError::NotFound(uri)) })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move { Err(PromptError::NotFound(prompt_name)) })
    }
}

pub async fn start_server(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let router = RouterService(TodoRouter::new(app));

    let server = Server::new(router);
    let transport = ByteTransport::new(stdin(), stdout());

    Ok(server.run(transport).await?)
}
