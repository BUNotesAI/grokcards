import { useEffect, useState, useRef, useCallback } from "react";
import { commands, type Todo, type TodoFilter } from "./bindings";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { Trash2, CheckCheck } from "lucide-react";

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

  const activeCount = todos.filter((t) => !t.completed).length;
  const completedCount = todos.filter((t) => t.completed).length;
  const allCompleted = todos.length > 0 && todos.every((t) => t.completed);

  return (
    <div className="flex min-h-screen items-start justify-center bg-background p-8">
      <Card className="w-full max-w-lg">
        <CardHeader className="pb-4">
          <CardTitle className="text-center text-2xl font-light tracking-wide">
            todos
          </CardTitle>
          <div className="flex gap-2 pt-2">
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
        </CardHeader>

        {todos.length > 0 && (
          <CardContent className="space-y-1 px-4 pb-4">
            {todos.map((todo) => (
              <div
                key={todo.id}
                className="group flex items-center gap-3 rounded-md px-2 py-2 hover:bg-muted/50"
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
          </CardContent>
        )}

        {todos.length > 0 && (
          <CardFooter className="flex items-center justify-between border-t px-4 py-3">
            <Badge variant="secondary">
              {activeCount} {activeCount === 1 ? "item" : "items"} left
            </Badge>

            <div className="flex gap-1">
              {(["All", "Active", "Completed"] as const).map((f) => (
                <Button
                  key={f}
                  variant={filter === f ? "secondary" : "ghost"}
                  size="sm"
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
                onClick={handleClearCompleted}
              >
                Clear completed
              </Button>
            )}
          </CardFooter>
        )}
      </Card>
    </div>
  );
}

export default App;
