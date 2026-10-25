---
name: spec-plan
description: Turn clarified requirements into a precise specification and implementation plan. Use after an interrogation phase or when a task needs explicit scope, contracts, acceptance criteria, risks, and ordered work.
disable-model-invocation: true
---

# Specification and Plan

Сформируй документ в следующем порядке:

1. **Контекст и цель** — какую проблему решаем.
2. **Scope** — что входит и что не входит.
3. **Контракты** — API, данные, состояния, ошибки и совместимость.
4. **Критерии приёмки** — проверяемые условия в формате Given / When / Then, где уместно.
5. **Риски и решения** — неизвестные, зависимости и выбранные defaults.
6. **План** — нумерованные небольшие шаги с файлами и способом проверки.

Не начинай реализацию, пока пользователь не подтвердит план. План должен быть достаточно конкретным, чтобы тесты можно было написать до production-кода.
