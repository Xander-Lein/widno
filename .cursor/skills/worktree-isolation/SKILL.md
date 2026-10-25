---
name: worktree-isolation
description: Plan safe parallel agent work with Git branches and worktrees. Use when independent subtasks may run concurrently or when a change is risky to the main working tree.
disable-model-invocation: true
---

# Worktree Isolation

Используй отдельную ветку и Git worktree для независимой или рискованной подзадачи.

Перед началом:

- проверь чистоту рабочей директории;
- назови ветку по задаче;
- зафиксируй границы изменяемых файлов;
- определи команды проверки.

После завершения:

- запусти проверки внутри worktree;
- сообщи commit и статус тестов;
- интегрируй изменения только после проверки конфликтов;
- удали worktree после слияния, если он больше не нужен.

Не выполняй две конкурентные задачи в одном рабочем каталоге без явного разрешения.
