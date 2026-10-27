# Widno

Компактный Windows-просмотрщик изображений на Rust, вдохновлённый
наблюдаемыми возможностями FastStone MaxView 3.3.

## Возможности текущего MVP

- frameless desktop window на `eframe/egui`;
- автоматическое скрытие верхней панели;
- drag-and-drop и путь к файлу первым аргументом;
- fit-to-window, 100% и zoom клавишами `+`/`-`;
- навигационная модель с циклическим переходом;
- common image formats через `image`: JPEG, PNG, GIF, BMP, TIFF, WebP, ICO и PNM/TGA;
- безопасный статический просмотр SVG;
- виртуальный просмотр изображений из ZIP, 7z и RAR без ручного извлечения;
- RAW preview через `rawloader`;
- опциональный PDF first-page rendering через PDFium runtime;
- EXIF extraction API;
- frameless borderless-окно с native Windows Snap (`Win` + стрелки), без глобального low-level keyboard hook;
- Ctrl+P: настройки форматов по умолчанию и пунктов Widno в контекстном меню;
- RAM-only LRU-кэш изображений с предварительной загрузкой следующего файла.

## Сборка

```powershell
cargo test
cargo run --release -- path\to\image.jpg
```

PDF отключён в стандартной Windows-сборке, чтобы не увеличивать размер и
время запуска приложения. Для отдельной сборки с PDF используйте
`--features pdf` и положите совместимый `pdfium.dll` рядом с приложением.

При первом запуске откроются настройки. Их можно открыть повторно сочетанием
`Ctrl+P`. Widno не перезаписывает существующий `UserChoice` Windows для
приложения по умолчанию; выбор default-приложения подтверждается пользователем
через системные настройки Windows.

Для регистрации `Open with Widno` запустите PowerShell-скрипт от имени текущего
пользователя:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\register-file-associations.ps1 `
  -Binary .\target\release\widno.exe
```

## Ограничения

RAW preview в первой версии использует безопасный grayscale fallback; полноценный
demosaicing и цветовой профиль являются отдельной задачей. Поддержка PDF зависит
от внешнего PDFium runtime. Пароли архивов пока доступны на API-уровне, а не
через интерактивный prompt.
