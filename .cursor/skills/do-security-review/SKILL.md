---
name: do-security-review
description: Review security-sensitive changes for secrets, access boundaries, data exposure, unsafe commands, dependencies, and bypasses. Use for authentication, authorization, CI, infrastructure, privacy, secret handling, or security-triggered reviews.
disable-model-invocation: true
---

# Security review

## Чеклист

- [ ] 1. Зафиксировать base, head, diff и security-триггер проверки.
- [ ] 2. Определить затронутые секреты, identities, permissions, данные и trust boundaries.
- [ ] 3. Проверить аутентификацию, авторизацию, валидацию входов и обходы ограничений.
- [ ] 4. Проверить утечки секретов в код, логи, ошибки, артефакты и историю Git.
- [ ] 5. Проверить shell-команды, CI/CD, hooks, зависимости, загрузки файлов и внешние интеграции.
- [ ] 6. Проверить negative paths: отказ, подмена, повтор запроса, privilege escalation и частичный сбой.
- [ ] 7. Подтвердить каждый finding фактическим кодом, трассировкой или воспроизводимой проверкой.
- [ ] 8. Для каждого finding указать severity, impact, evidence, remediation и статус.
- [ ] 9. Запустить релевантные security-тесты и обычные проверки после исправлений.
- [ ] 10. Передать отдельный security-отчёт в `do-review`; не считать отсутствие findings доказательством безопасности.

## Правила

- Не печатай и не сохраняй реальные секреты во время проверки.
- Не выполняй destructive или production-действия для подтверждения гипотезы.
- Не смешивай security findings со style, usability и обычными дефектами.
- Для high-risk findings явно запроси human approval и укажи план rollback.
