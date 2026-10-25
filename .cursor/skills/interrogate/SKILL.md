---
name: interrogate
description: Conduct a focused requirements interview before implementation. Use for new features, bug fixes, refactors, or any task with unclear scope, inputs, outputs, edge cases, or validation criteria.
disable-model-invocation: true
---

# Interrogate

Задай 3–5 конкретных вопросов и дождись ответов до написания основного кода.

Покрой только неизвестные, которые влияют на реализацию:

- цель и границы задачи;
- входные и выходные данные;
- пользовательские сценарии и граничные случаи;
- ограничения архитектуры, совместимости и безопасности;
- критерии приёмки и способ проверки.

Не задавай вопросы, на которые уже есть ответ в репозитории или запросе. После ответов кратко повтори понятые требования и предложи перейти к `spec-plan`.
