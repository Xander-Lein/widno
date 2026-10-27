# Widno 1.0.0

![Widno icon](assets/widno-icon.png)

Компактный быстрый просмотрщик изображений для Windows x64 на Rust. Widno
открывает изображения без лишних панелей, умеет листать папки и просматривать
изображения прямо внутри архивов.

[![Latest Release](https://img.shields.io/github/v/release/Xander-Lein/widno?display_name=tag&sort=semver)](https://github.com/Xander-Lein/widno/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Скачать

Откройте страницу [Releases](https://github.com/Xander-Lein/widno/releases) и
скачайте `Widno-1.0.0-Windows-x64-Setup.exe`.

Установщик рассчитан на Windows 10/11 x64. Исходники доступны через кнопку
`Source code (zip)` на странице релиза.

## Возможности

- безрамочное окно с нативным Windows Snap (`Win` + стрелки);
- быстрый просмотр папок и переход стрелками;
- JPEG, PNG, GIF, BMP, TIFF, WebP, ICO, PNM, TGA и статический SVG;
- RAW preview распространённых форматов камер;
- просмотр изображений внутри ZIP, 7z и RAR без ручного извлечения;
- zoom с шагом 25%, pan мышью и поддержка колеса/тачпада;
- EXIF и технические метаданные;
- сохранение через `Ctrl+S` в PNG, JPEG, WebP, BMP или TIFF;
- RAM-only LRU-кэш и предварительная загрузка следующего изображения;
- иконка Widno в EXE, ярлыках и файловых ассоциациях.

PDF отключён в стандартной сборке, чтобы не увеличивать размер и время запуска.

## Управление

| Действие | Клавиша |
| --- | --- |
| Следующее/предыдущее изображение | `Right` / `Left` |
| Увеличить/уменьшить | `Ctrl +` / `Ctrl -` |
| Сбросить масштаб и центр | `Ctrl+0` |
| Сохранить как | `Ctrl+S` |
| Настройки | `Ctrl+P` |
| Закрыть | `Esc` или `Alt+F4` |
| Переместить увеличенное изображение | зажать ЛКМ и перетащить |

## Установка и ассоциации

Установщик добавляет Widno в меню «Пуск». Ассоциации и пункт
`Browse with Widno` можно настроить в `Ctrl+P`. Уже выбранное пользователем
приложение Windows не перезаписывается.

Для ручной регистрации текущего EXE:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\register-file-associations.ps1 `
  -Binary .\target\release\widno.exe
```

## Сборка из исходников

Нужны Rust stable и инструменты сборки Windows x64:

```powershell
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --locked
```

Результат: `target\release\widno.exe`.

Для создания установщика нужен [Inno Setup 6](https://jrsoftware.org/isinfo.php):

```powershell
.\scripts\build-installer.ps1
```

## Ограничения

RAW preview использует безопасный fallback без полноценного demosaicing.
Пароли архивов доступны через API; интерактивное окно ввода пароля ещё не
включено.

## English

Widno is a compact, fast Windows x64 image viewer written in Rust. It provides
a borderless UI, native Windows Snap support, folder navigation, archive image
browsing, zoom/pan, metadata, export, and a RAM-only LRU cache.

Download `Widno-1.0.0-Windows-x64-Setup.exe` from
[GitHub Releases](https://github.com/Xander-Lein/widno/releases). The GitHub
release page also provides the automatically generated `Source code (zip)`.

Supported formats include JPEG, PNG, GIF, BMP, TIFF, WebP, ICO, SVG, common RAW
formats, ZIP, 7z and RAR image archives. PDF is disabled in the default build
to keep startup time and package size small.

Build with `cargo test --locked` and `cargo build --release --locked`. Run
`scripts/build-installer.ps1` after installing Inno Setup 6.
