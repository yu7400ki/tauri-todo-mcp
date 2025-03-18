import { load, Store } from "@tauri-apps/plugin-store";
import { useEffect, useRef, useState } from "react";
import styles from "./App.module.css";

type Todo = {
  id: number;
  text: string;
  done: boolean;
};

const store_path = "store.json";
const todos_key = "todos";

export default function App() {
  const storeRef = useRef<Store | null>(null);
  const [todos, setTodos] = useState<Todo[]>([]);

  useEffect(() => {
    const init = async () => {
      const store = await load(store_path, { autoSave: true });
      storeRef.current = store;

      const todos = await store.get<Todo[]>(todos_key);
      setTodos(todos ?? []);

      const unlisten = await store.onKeyChange<Todo[]>(todos_key, (todos) => {
        setTodos(todos ?? []);
      });

      const id = setInterval(async () => {
        await store.reload();
        const todos = await store.get<Todo[]>(todos_key);
        setTodos(todos ?? []);
      }, 1000);

      return () => {
        clearInterval(id);
        unlisten();
      };
    };

    const p = init();
    return () => {
      p.then((cleanup) => cleanup());
    };
  }, []);

  const addTodo = async (text: string) => {
    const newTodo = { id: Date.now(), text, done: false };
    await storeRef.current?.set(todos_key, [...todos, newTodo]);
  };

  const removeTodo = async (id: number) => {
    await storeRef.current?.set(
      todos_key,
      todos.filter((todo) => todo.id !== id)
    );
  };

  const updateTodo = async (todo: Todo) => {
    await storeRef.current?.set(
      todos_key,
      todos.map((t) => (t.id === todo.id ? todo : t))
    );
  };

  return (
    <div className={styles.container}>
      <h1 className={styles.title}>Todos</h1>
      <input
        type="text"
        className={styles.todoInput}
        placeholder="Add todo"
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            addTodo(e.currentTarget.value);
            e.currentTarget.value = "";
          }
        }}
      />
      <ul className={styles.todoList}>
        {todos.map((todo) => (
          <li key={todo.id} className={styles.todoItem}>
            <label className={styles.todoText} data-done={todo.done}>
              <input
                type="checkbox"
                className={styles.checkbox}
                checked={todo.done}
                onChange={(e) =>
                  updateTodo({ ...todo, done: e.target.checked })
                }
              />
              {todo.text}
            </label>
            <button
              className={styles.removeButton}
              onClick={() => removeTodo(todo.id)}
            >
              Remove
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
