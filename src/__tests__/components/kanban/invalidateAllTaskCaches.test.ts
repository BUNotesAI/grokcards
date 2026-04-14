import { describe, it, expect, vi } from "vitest";
import { QueryClient } from "@tanstack/react-query";
import { invalidateAllTaskCaches } from "@/components/kanban/invalidateAllTaskCaches";

describe("invalidateAllTaskCaches", () => {
  it("指定 wb 时 invalidate tasks-kanban + 精确 (tasks, wb) / (positions, wb)", () => {
    const qc = new QueryClient();
    const spy = vi.spyOn(qc, "invalidateQueries");

    invalidateAllTaskCaches(qc, "projects/test");

    expect(spy).toHaveBeenCalledWith({ queryKey: ["tasks-kanban"] });
    expect(spy).toHaveBeenCalledWith({
      queryKey: ["tasks", "projects/test"],
    });
    expect(spy).toHaveBeenCalledWith({
      queryKey: ["positions", "projects/test"],
    });
  });

  it("未指定 wb 时 fallback 到 broad key", () => {
    const qc = new QueryClient();
    const spy = vi.spyOn(qc, "invalidateQueries");

    invalidateAllTaskCaches(qc);

    expect(spy).toHaveBeenCalledWith({ queryKey: ["tasks-kanban"] });
    expect(spy).toHaveBeenCalledWith({ queryKey: ["tasks"] });
    expect(spy).toHaveBeenCalledWith({ queryKey: ["positions"] });
  });
});
