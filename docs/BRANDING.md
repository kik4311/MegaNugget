# Логотип MegaNugget

Единственный редактируемый исходник: `assets/branding/meganugget-logo.svg`.
Это собственный знак MegaNugget, не ресурс GoldenNugget или Apple.

## Где используется

- Desktop: боковая панель, иллюстрация на главной, заголовок «О проекте».
  Vite импортирует исходный SVG и упаковывает его локально — внешних запросов нет.
- Web: SVG favicon, ICO fallback и Apple touch icon в `public/branding`.
- Tauri: PNG 32/128/256/512, Windows ICO и macOS ICNS. Пути явно указаны
  в `tauri.conf.json`; упаковка дистрибутивов по-прежнему отключена.
- iOS: AppIcon с явными iPhone slots для iOS 15 и маркетинговой иконкой 1024;
  AppLogo 1x/2x/3x на главной странице и в разделе «О проекте».
  AppIcon непрозрачный, без alpha; скругление выполняет iOS.
- README: исходный SVG.

## Обновление

Для генерации нужны Python 3, Pillow и `rsvg-convert` (пакет `librsvg2-bin`
на Debian/Ubuntu). Это инструменты разработки, не зависимости приложений.

```sh
python3 scripts/generate_branding.py
python3 -m unittest discover -s tests -p 'test_*.py' -v
```

Сгенерированные файлы коммитятся. `assets/branding/generated.json` содержит
SHA-256 исходника и каждого результата; тесты ловят несогласованные изменения,
проверяют размеры/непрозрачность иконок iOS и контейнеры ICO/ICNS.
Точное бинарное воспроизведение зависит от версии librsvg/Pillow; после смены
инструментов обновляйте результаты и manifest одним запуском генератора.

Кеши favicon/иконок ОС могут показывать прежнее изображение до перезапуска
или переустановки приложения. Уже установленный IPA сам по себе не обновится.
