# Нативное Rust-приложение

## Статус первого этапа

Нативный Windows GUI находится в `crates/factory-atlas-app/`. Он использует eframe/egui с `winit` и OpenGL (`glow`), а операции над проектом и формат `.factoryatlas` — из `factory-atlas-core`.

На первом этапе пользователь может:

- начать с полностью пустого проекта;
- создать предмет или жидкость, задав собственный ID и RU/EN названия;
- добавить узлы источника, цели и заметки;
- указать темп источника/цели и связать узлы только при совпадении ресурса и единиц;
- открыть, сохранить и повторно открыть существующий формат `.factoryatlas` (в том числе мигрировать v1 через общий парсер core).

Машины, рецепты, визуальный поиск, 3D-просмотр, импорт `.jar` и просмотр `.nbt` пока не перенесены в GUI. Эти функции остаются в старом веб-интерфейсе до отдельных проверенных этапов миграции.

## Сборка для разработки

Требуется стабильный Rust и, для MSVC target на Windows, Visual Studio C++ Build Tools с Windows SDK. Сборка/запуск из checkout:

```powershell
cargo run -p factory-atlas-app
cargo build -p factory-atlas-app --release
```

Release-бинарник создаётся в `target/release/factory-atlas.exe`. В этой разработческой версии нет установщика и portable ZIP. Их нельзя объявлять готовыми, пока артефакт не упакован и не проверен на Windows.

## Runtime-зависимости GUI

Windows GUI использует только нативные зависимости eframe/egui, `winit`, `glow` и Windows native file dialogs (`rfd`). Для сборки `eframe` выключены web/WASM renderer features; `egui-winit` `links` feature выключен. В целевой Windows dependency tree не должно быть Tauri, Wry, WebView2 или browser launcher. Проверьте это командой:

```powershell
cargo tree -p factory-atlas-app --target x86_64-pc-windows-msvc -e normal
```

Старые `src/`, `src-tauri/`, `package.json`, Svelte, TypeScript, Vite и веб-проверки пока остаются необходимыми для существующей версии приложения. Удаление этих частей планируется отдельным этапом после проверки нативного GUI и переноса оставшихся пользовательских функций.
