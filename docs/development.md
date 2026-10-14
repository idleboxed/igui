# Разработка igui

Следуйте [AGENTS.md](../AGENTS.md) и [IDLE.md](../IDLE.md).

## Features

- `desktop` — включена по умолчанию; окно X11.
- `rgb565` — включена по умолчанию; DRM-буферы RGB565 вместо XRGB8888.
- `drm` — вывод через Linux DRM/KMS без оконной системы.
- `drm-diagnostics` — измерения операций DRM/KMS.

## Исходники

- [`runtime`](../src/runtime/mod.rs) — запуск, цикл меню и передача игры менеджеру.
- [`model`](../src/model/mod.rs) — каталог, состояние меню, поиск и фильтры.
- [`render`](../src/render/mod.rs) — панели, превью и диалоги.
- [`storage`](../src/storage/mod.rs) — чтение данных, предпочтения и снимки экрана.
- [`input`](../src/input/mod.rs) — геймпады и правила навигации.
- [`display`](../src/display/mod.rs) — оконный и DRM-вывод.
- [`ipc`](../src/ipc/mod.rs) — команды локального IPC.

## Проверки

Проверки в [`tests/`](../tests/) сгруппированы по компонентам исходников:
модель, отрисовка, ввод, IPC, runtime и хранение данных. Общий вход —
[`main.rs`](../tests/main.rs), помощники — [`common/`](../tests/common/),
данные — [`_fixtures/`](../tests/_fixtures/).
Из каталога `igui` для выбора компонента используйте фильтр имени, например:

```sh
cargo test --locked --test integration input::
```

Из каталога `igui`:

```sh
cargo test --locked -- --test-threads=1
cargo test --locked --all-features -- --test-threads=1
cargo check --locked --no-default-features --all-targets
```

Тестам нужны доступный для записи `/dev/shm` и временный каталог.
Оборудование, игры и соседние приложения не требуются.
[GitHub Actions](../.github/workflows/rust-package.yml) также проверяет компиляцию
для ARMv7 musl. Тесты на ПК не подтверждают работу на конкретном устройстве.
