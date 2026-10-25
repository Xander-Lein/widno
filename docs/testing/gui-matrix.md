# Widno GUI smoke matrix

Запускать из корня проекта после `cargo build --release`:

```powershell
.\target\release\widno.exe .\tests\fixtures\images\fixture.png
```

## Сценарии

| ID | Сценарий | Ожидаемый результат |
|---|---|---|
| GUI-01 | Запустить `widno.exe` | Открывается одно borderless fullscreen окно, консольного окна нет |
| GUI-02 | Открыть JPEG/PNG/GIF/BMP/TIFF/WebP | Изображение отображается, верхняя строка показывает имя, размер, разрешение, N/M и zoom |
| GUI-03 | Открыть папку `tests/fixtures/images` | Стрелки вправо/влево листают только поддерживаемые изображения; N/M меняется |
| GUI-04 | Открыть `tests/fixtures/archives/фотки.rar` | Список/первое изображение архива открывается без извлечения в репозиторий |
| GUI-05 | Открыть `tests/fixtures/archives/protected.7z` | Неверный пароль отклоняется, `widno-test` открывает entry; пароль не сохраняется |
| GUI-06 | Открыть многостраничный PDF | Up/Down/PageUp/PageDown листают страницы и останавливаются на границах |
| GUI-07 | Нажать Ctrl+Plus/Ctrl+Minus/Ctrl+0 и использовать wheel/touchpad | Zoom меняется в процентах, Ctrl+0 возвращает 100% |
| GUI-08 | При zoom больше 100% перетащить изображение мышью | Viewport перемещается без ускоряющегося накопления delta |
| GUI-09 | Нажать Ctrl+S и выбрать PNG/JPEG/WebP/BMP/TIFF | Save As создаёт декодируемый файл, оригинал не изменяется |
| GUI-10 | Нажать Esc | Приложение закрывается |

## Автоматизированные проверки

- `cargo test` проверяет byte-identical RAR, RAR listing, generated common formats
  и password-protected 7z.
- GUI-01–GUI-10 требуют native Windows desktop smoke run; снимок экрана и
  результат каждого сценария фиксируются в task evidence, а не заменяются
  одной проверкой компиляции.
