# Widno

Компактный Windows-просмотрщик изображений на Rust, вдохновлённый
наблюдаемыми возможностями FastStone MaxView 3.3.

## Возможности текущего MVP

- frameless desktop window на `eframe/egui`;
- автоматическое скрытие верхней панели;
- drag-and-drop и путь к файлу первым аргументом;
- fit-to-window, 100%, zoom клавишами `+`/`-`, fullscreen `F11`;
- навигационная модель с циклическим переходом;
- common image formats через `image`;
- открытие первого изображения из ZIP, 7z и RAR без ручного извлечения;
- RAW preview через `rawloader`;
- PDF first-page rendering через PDFium runtime;
- EXIF extraction API.

## Сборка

```powershell
cargo test
cargo run --release -- path\to\image.jpg
```

Для PDF положите совместимый `pdfium.dll` рядом с приложением или задайте
`WIDNO_PDFIUM_PATH` каталогом, в котором он находится.

## Ограничения

RAW preview в первой версии использует безопасный grayscale fallback; полноценный
demosaicing и цветовой профиль являются отдельной задачей. Поддержка PDF зависит
от внешнего PDFium runtime. Пароли архивов пока доступны на API-уровне, а не
через интерактивный prompt.
