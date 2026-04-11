import { useEffect, useState, useRef, useCallback, useMemo } from "react";
import { commands, type Todo, type TodoFilter } from "@/bindings";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { Trash2, CheckCheck } from "lucide-react";

export default function TodoPage() {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [filter, setFilter] = useState<TodoFilter>("All");
  const [newTitle, setNewTitle] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editText, setEditText] = useState("");
  const editRef = useRef<HTMLInputElement>(null);

  // 始终加载全部 todo，客户端做 view 层过滤
  const loadTodos = useCallback(async () => {
    const result = await commands.listTodos("All");
    if (result.status === "ok") {
      setTodos(result.data);
    }
  }, []);

  useEffect(() => {
    loadTodos();
  }, [loadTodos]);

  // 根据 filter 过滤展示列表（纯 view 层）
  const filteredTodos = useMemo(() => {
    switch (filter) {
      case "Active":
        return todos.filter((t) => !t.completed);
      case "Completed":
        return todos.filter((t) => t.completed);
      default:
        return todos;
    }
  }, [todos, filter]);

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

  // 计数始终基于全量数据，不受 filter 影响
  const activeCount = todos.filter((t) => !t.completed).length;
  const completedCount = todos.filter((t) => t.completed).length;
  const allCompleted = todos.length > 0 && todos.every((t) => t.completed);

  return (
    <div className="mx-auto max-w-2xl">
      <div className="mb-5 flex items-center justify-between">
        <h2 className="font-heading text-2xl">Todo</h2>
        <div className="flex gap-4 text-xs text-muted-foreground">
          <span><strong className="font-semibold text-foreground">{activeCount}</strong> active</span>
          <span><strong className="font-semibold text-foreground">{completedCount}</strong> completed</span>
        </div>
      </div>

      <div className="mb-4 flex gap-2">
        {todos.length > 0 && (
          <Button
            variant={allCompleted ? "default" : "outline"}
            size="icon"
            onClick={handleToggleAll}
            title="Toggle all"
          >
            <CheckCheck className="size-4" />
          </Button>
        )}
        <Input
          placeholder="What needs to be done?"
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleCreate();
          }}
          autoFocus
          className="flex-1"
        />
      </div>

      {filteredTodos.length > 0 && (
        <div className="overflow-hidden rounded-lg border bg-card shadow-sm">
          {filteredTodos.map((todo) => (
            <div
              key={todo.id}
              className="group flex items-center gap-3 border-b px-3.5 py-2.5 last:border-b-0 hover:bg-muted/30"
            >
              {editingId === todo.id ? (
                <Input
                  ref={editRef}
                  value={editText}
                  onChange={(e) => setEditText(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") submitEdit(todo.id);
                    if (e.key === "Escape") cancelEdit();
                  }}
                  onBlur={() => submitEdit(todo.id)}
                  className="flex-1"
                />
              ) : (
                <>
                  <Checkbox
                    checked={todo.completed}
                    onCheckedChange={() => handleToggle(todo.id)}
                  />
                  <span
                    className={`flex-1 cursor-pointer text-sm ${
                      todo.completed
                        ? "text-muted-foreground line-through"
                        : ""
                    }`}
                    onDoubleClick={() => startEditing(todo)}
                  >
                    {todo.title}
                  </span>
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    className="opacity-0 group-hover:opacity-100"
                    onClick={() => handleDelete(todo.id)}
                  >
                    <Trash2 className="size-3.5 text-muted-foreground" />
                  </Button>
                </>
              )}
            </div>
          ))}
        </div>
      )}

      {todos.length > 0 && (
        <div className="mt-3 flex items-center justify-between">
          <Badge variant="secondary">
            {activeCount} {activeCount === 1 ? "item" : "items"} left
          </Badge>

          <div className="flex gap-0.5 rounded-lg bg-muted p-0.5">
            {(["All", "Active", "Completed"] as const).map((f) => (
              <Button
                key={f}
                variant={filter === f ? "secondary" : "ghost"}
                size="sm"
                className={filter === f ? "bg-card shadow-sm font-semibold" : ""}
                onClick={() => setFilter(f)}
              >
                {f}
              </Button>
            ))}
          </div>

          {completedCount > 0 && (
            <Button
              variant="ghost"
              size="sm"
              className="text-muted-foreground hover:text-destructive"
              onClick={handleClearCompleted}
            >
              Clear completed
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
