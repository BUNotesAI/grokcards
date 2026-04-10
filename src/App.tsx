import { useEffect, useState, useRef, useCallback } from "react";
import { commands, type Todo, type TodoFilter } from "./bindings";
import "./App.css";

function App() {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [filter, setFilter] = useState<TodoFilter>("All");
  const [newTitle, setNewTitle] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editText, setEditText] = useState("");
  const editRef = useRef<HTMLInputElement>(null);

  const loadTodos = useCallback(async () => {
    const result = await commands.listTodos(filter);
    if (result.status === "ok") {
      setTodos(result.data);
    }
  }, [filter]);

  useEffect(() => {
    loadTodos();
  }, [loadTodos]);

  const handleCreate = async () => {
    const title = newTitle.trim();
    if (!title) return;
    const result = await commands.createTodo(title);
    if (result.status === "ok") {
      setNewTitle("");
      loadTodos();
    }
  };

  const handleToggle = async (id: number) => {
    const result = await commands.toggleTodo(id);
    if (result.status === "ok") {
      loadTodos();
    }
  };

  const handleDelete = async (id: number) => {
    const result = await commands.deleteTodo(id);
    if (result.status === "ok") {
      loadTodos();
    }
  };

  const handleToggleAll = async () => {
    const allCompleted = todos.length > 0 && todos.every((t) => t.completed);
    const result = await commands.toggleAll(!allCompleted);
    if (result.status === "ok") {
      setTodos(result.data);
    }
  };

  const handleClearCompleted = async () => {
    const result = await commands.clearCompleted();
    if (result.status === "ok") {
      setTodos(result.data);
    }
  };

  const startEditing = (todo: Todo) => {
    setEditingId(todo.id);
    setEditText(todo.title);
  };

  useEffect(() => {
    if (editingId !== null && editRef.current) {
      editRef.current.focus();
    }
  }, [editingId]);

  const submitEdit = async (id: number) => {
    const title = editText.trim();
    if (!title) {
      await commands.deleteTodo(id);
    } else {
      await commands.updateTodo(id, title);
    }
    setEditingId(null);
    loadTodos();
  };

  const cancelEdit = () => {
    setEditingId(null);
  };

  // 从完整列表计算活跃数（filter=All 时直接用，否则需单独请求）
  // 简化处理：始终显示当前列表的统计
  const activeCount = todos.filter((t) => !t.completed).length;
  const completedCount = todos.filter((t) => t.completed).length;
  const allCompleted = todos.length > 0 && todos.every((t) => t.completed);

  return (
    <section className="todoapp">
      <header className="header">
        <h1>todos</h1>
        <input
          className="new-todo"
          placeholder="What needs to be done?"
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleCreate();
          }}
          autoFocus
        />
      </header>

      {todos.length > 0 && (
        <section className="main">
          <input
            id="toggle-all"
            className="toggle-all"
            type="checkbox"
            checked={allCompleted}
            onChange={handleToggleAll}
          />
          <label htmlFor="toggle-all">Mark all as complete</label>
          <ul className="todo-list">
            {todos.map((todo) => (
              <li
                key={todo.id}
                className={[
                  todo.completed ? "completed" : "",
                  editingId === todo.id ? "editing" : "",
                ]
                  .join(" ")
                  .trim()}
              >
                <div className="view">
                  <input
                    className="toggle"
                    type="checkbox"
                    checked={todo.completed}
                    onChange={() => handleToggle(todo.id)}
                  />
                  <label onDoubleClick={() => startEditing(todo)}>
                    {todo.title}
                  </label>
                  <button
                    className="destroy"
                    onClick={() => handleDelete(todo.id)}
                  />
                </div>
                {editingId === todo.id && (
                  <input
                    ref={editRef}
                    className="edit"
                    value={editText}
                    onChange={(e) => setEditText(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") submitEdit(todo.id);
                      if (e.key === "Escape") cancelEdit();
                    }}
                    onBlur={() => submitEdit(todo.id)}
                  />
                )}
              </li>
            ))}
          </ul>
        </section>
      )}

      {todos.length > 0 && (
        <footer className="footer">
          <span className="todo-count">
            <strong>{activeCount}</strong>{" "}
            {activeCount === 1 ? "item" : "items"} left
          </span>
          <ul className="filters">
            {(["All", "Active", "Completed"] as const).map((f) => (
              <li key={f}>
                <a
                  href="#"
                  className={filter === f ? "selected" : ""}
                  onClick={(e) => {
                    e.preventDefault();
                    setFilter(f);
                  }}
                >
                  {f}
                </a>
              </li>
            ))}
          </ul>
          {completedCount > 0 && (
            <button className="clear-completed" onClick={handleClearCompleted}>
              Clear completed
            </button>
          )}
        </footer>
      )}
    </section>
  );
}

export default App;
