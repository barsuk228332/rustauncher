# Ассеты rustauncher

| Файл | Что это |
|---|---|
| `logo.svg` | исходный векторный логотип 512×512 |
| `rustauncher.png` | логотип 512×512 для README и установки |
| `icon-256.png` … `icon-32.png` | иконки приложения разных размеров |
| `banner.svg` / `banner.png` | баннер 1600×520 для README |
| `empty-apps.svg`, `empty-prefixes.svg`, `empty-search.svg` | иллюстрации пустых состояний |
| `rustauncher.desktop` | ярлык для меню приложений |

## Установка иконки и ярлыка

```bash
# иконка в тему hicolor
install -Dm644 assets/icon-256.png \
  ~/.local/share/icons/hicolor/256x256/apps/rustauncher.png

# ярлык в меню приложений
install -Dm644 assets/rustauncher.desktop \
  ~/.local/share/applications/rustauncher.desktop

update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

Бинарь `rustauncher` должен лежать в `PATH` (например, `~/.cargo/bin/`
или `/usr/local/bin/`). Иконка окна при этом уже встроена в приложение.

## Перегенерация PNG из SVG

Нужен ImageMagick с делегатом librsvg:

```bash
cd assets
magick -background none logo.svg -resize 512x512 rustauncher.png
for s in 256 128 64 48 32; do
  magick -background none logo.svg -resize ${s}x${s} icon-${s}.png
done
magick -background none banner.svg -depth 8 banner.png
for n in apps prefixes search; do
  magick -background none empty-${n}.svg -depth 8 empty-${n}.png
done
```
