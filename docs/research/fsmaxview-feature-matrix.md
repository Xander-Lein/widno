# FSMaxView 3.3: feature matrix

Статический research выполнен 24 октября 2026 года по
`FSMaxView3.3/FSMaxViewSetup33.exe`. Бинарник и извлечённые файлы не
запускались.

## Evidence

- Файл является NSIS-2 installer, сжатым LZMA.
- Установщик не имеет цифровой подписи.
- Внутри присутствуют `MaxView.exe` (4,680,192 bytes), `MaxViewHelp.chm`,
  пять `fsplugin*.dll`, `unrar.dll`, `7z.dll` и несколько `.db` с иконками.
- Строки `FastStone MaxView` и `FastStone MaxView 3.3` подтверждают
  идентичность продукта.
- Строки и зависимости указывают на Delphi/VCL-компоненты (`Toolbar2000`,
  `TntMenus`, `ImageEn`) и не являются исходным кодом, который можно
  переносить в Rust.

## Matrix

| Возможность | Статус | Evidence / next check |
|---|---|---|
| Просмотр изображения | confirmed statically | `TFIMAGEVIEWER`, `ImageEn`, image format strings |
| Оконный и полноэкранный режимы | confirmed statically | `Windowed View`, `Full Screen`, `BntFullScreen` |
| Автоскрытие интерфейса/курсора | confirmed statically | `AutoHideMouseCursorInFullScreen`, toolbar/menu strings |
| Навигация next/previous | confirmed statically | `BntNext`, `BntPrevious`, `BottomForNextPrev` |
| Навигация по папкам | confirmed statically | `Jump to next folder when reaching the end of current folder` |
| Zoom колесом и клавишами | confirmed statically | `Scroll mouse wheel to zoom in / out`, `+/-`, `1..9,0` |
| Панорамирование/сброс позиции | confirmed statically | `Reset image position to (0,0)`, `Image Zoom Ratio` |
| Лупа/выделение области | confirmed statically | `Extract Selected Area as New Image`, `CRMYZOOM` |
| EXIF и свойства изображения | confirmed statically | `Image Properties including EXIF metadata`, `Auto - Rotate by EXIF orientation tag` |
| JPEG, PNG, GIF, BMP, TIFF | confirmed statically | format filters and decoder strings |
| ICO, CUR, PCX, JP2/J2K, TGA, PPM/PGM/PBM, WBMP | confirmed statically | `All graphics` filter strings |
| PSD, WMF, EMF | confirmed statically | `All graphics` filter strings |
| RAW CRW/CR2/CR3, NEF/NRW, ORF, RAF, MRW, PEF, SRF/ARW/SR2, DNG | confirmed statically | RAW filters and settings strings |
| RAW quality/speed settings | confirmed statically | `RAW` settings help text and `full size` hint |
| PDF | confirmed statically | `Adobe PDF`, `UPDFOptions`, `*.pdf` |
| ZIP/RAR/7z | confirmed statically | `Show Archive (zip, rar, 7z) File List`, `7z.dll`, `unrar.dll` |
| CBR/CBZ/CB7 | confirmed statically | archive filter string |
| Other archive formats | confirmed statically as advertised | `ALL Archives` includes ACE, ARC, ARJ, CAB, GZ, TAR, ZIP and others; exact image-view behavior unknown |
| Password-protected archives | partially confirmed | password-related strings; successful behavior must be tested |
| Save/convert and batch image operations | confirmed statically | `Save all images in a single file`, save/error strings |
| Image editing/annotation | confirmed statically | crop, pencil, line highlighter, rotate/save prompts |
| Scanner acquisition | confirmed statically | `Acquire Images from Scanner` |
| External editor integration | confirmed statically | `Edit with External Program` |
| File deletion | confirmed statically | delete confirmation strings |
| System association | confirmed statically | `FastStone MaxView` under `Photo viewer` |

## Rust implementation impact

The initial user scope describes a viewer, but the binary advertises additional
editing, acquisition, conversion, deletion, association and external-editor
features. They remain out of the first viewer MVP until behavior is confirmed
and explicitly accepted.

The archive list demonstrates that `ZIP/RAR/7z` is not equivalent to supporting
all archives. The Rust implementation must expose per-format capability and
return an explicit unsupported result.

RAW and PDF support should be adapter-based. A Rust crate may wrap native
libraries or cover only a subset of formats; this must be recorded per fixture
instead of claiming universal support.

## Safe next investigation

1. Obtain owner-approved runtime test scenarios and run the original only in an
   isolated Windows environment.
2. Record window states, menus, shortcuts, supported extensions, archive
   password behavior and error messages as observed behavior, without copying
   proprietary code or resources.
3. Build a clean Rust fixture set from independently sourced or generated test
   files.
