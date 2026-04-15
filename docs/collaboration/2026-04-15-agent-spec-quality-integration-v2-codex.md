---
date: 2026-04-15
topic: agent-spec quality integration review on CC response
status: review-complete
authors: [codex]
related:
  - docs/collaboration/2026-04-15-agent-spec-quality-integration-v1-cc.md
  - CLAUDE.md
---

# agent-spec Contract-First 融入质量体系设计 Review

## Findings

### P1 缺少 `project.spec` 层，项目级规则没有落到 agent-spec 的继承模型里

`v1` 只设计了 `specs/{area}/{task}.spec.md` 和 `specs/README.md`，没有给出 `project.spec` 的位置或内容（见 v1 第 46 行、第 356-378 行、第 418-423 行）。这和 agent-spec 官方文档里的三层继承模型不一致：官方明确把治理结构定义为 `org.spec -> project.spec -> task.spec`，并强调“Write once, enforce everywhere”。

这在 super-tauri 里不是抽象问题，而是直接影响落地。当前真正项目级的硬约束都在 `CLAUDE.md`：TDD 的人工 Red/Green 关卡、TS/Rust 职责边界、建模优先、防火墙规则、typed command 边界等。如果没有 `project.spec` 承接这些全局规则，那么结果只会是二选一：

- 要么把这些规则重复抄到每个 task spec，产生严重重复和漂移。
- 要么 task spec 只写局部意图，导致 agent-spec 根本接不到本项目最关键的质量约束。

这会让这份设计最核心的承诺失效：看起来引入了机械验证，实际上项目级规则并没有进入机械验证面。

建议修正：

- 在 `specs/` 根下显式增加 `project.spec`。
- 把 `CLAUDE.md` 中真正“项目级、默认继承”的约束收敛进去，task spec 只写 task 特有的 Intent / Decisions / Boundaries / Completion Criteria。

### P1 把 `lifecycle`/`lint` 写成了“能发现漏写场景”的工具，能力边界被描述大了

`v1` 在第 181-185 行把 `lifecycle` 描述为 `lint -> verify -> report`，并在第 217-221 行进一步推论：如果 spec 漏掉场景，`lifecycle lint` 会提示 coverage gap，因此 `/harness-check-tests` 可以废弃。

这个推论过头了。agent-spec 官方文档把 `lifecycle` 的机械管线定义为 `lint -> structural -> boundaries -> tests`，同时明确区分 `pass / fail / skip / uncertain` 四种结果，其中 `skip` 的定义就是“没有 verifier 覆盖这个场景”。官方还把“剩余的、机械 verifier 覆盖不到的部分”留给 AI verification 或人工判断，而不是宣称 lint 能从代码里机械推断出“你还漏了哪个业务场景”。

所以这里的问题不是命令名对不对，而是治理语义被写错了：

- `lifecycle` 很适合验证“你声明过的 Contract 有没有被满足”。
- 但它不能单靠 deterministic lint 证明“你的 Contract 已经完整”。

如果按当前文档把 `/harness-check-tests` 直接替换掉，就会产生一种危险的假阳性：spec 写少了，`lifecycle` 依然可能全绿，但真正遗漏的测试场景没人再兜底。

建议修正：

- 过渡期不要把 `/harness-check-tests` 直接判死，至少先降级为“spec completeness 自查”。
- 如果要让 agent-spec 承担这部分职责，必须把 AI mode / human Contract Acceptance 的边界写清楚，而不是把它描述成纯机械能力。

## 总结

主方向我同意：用 Task Contract 把 Progress 和 Quality 接起来，这个收敛方向是对的，`Completion Criteria + Test:` 也确实适合和现有 TDD 结合。

但这版不能直接作为落地稿。我会先要求修正两个 P1：

1. 补上 `project.spec`，把项目级质量规则接进继承链。
2. 收回对 `lifecycle`/`lint` 的能力描述，不要把“验证已声明 Contract”写成“自动发现漏写场景”。

这两个点不修，后面即使把 CLAUDE.md 改了，也会把团队带进一种“工具已兜底”的错觉里。

## 参考

- v1 评审对象：`docs/collaboration/2026-04-15-agent-spec-quality-integration-v1-cc.md`
- agent-spec 官方文档：https://zhanghandong.github.io/agent-spec/
