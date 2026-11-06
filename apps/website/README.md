# Сайт MegaNugget

Статический русскоязычный лендинг: HTML, CSS и небольшой JS для FAQ. Нет npm,
сборки, внешних шрифтов, аналитики, cookies, форм авторизации и сетевых API.
Без JavaScript всё содержимое и ссылки доступны, FAQ раскрывается штатно.

## Локальный запуск

Из корня репозитория:

```sh
python3 -m http.server 4174 --bind 127.0.0.1 --directory apps/website
```

Откройте http://127.0.0.1:4174.

## Публикация

Загрузите **содержимое** `apps/website` на статический хостинг: GitHub Pages,
Cloudflare Pages, Netlify или свой сервер. Build command не нужен. Все пути
к ресурсам относительные — сайт работает и в подпапке `/MegaNugget/`.
Автоматическая публикация и домен не настроены; без согласования сайт не деплоится.
После выбора домена можно добавить canonical URL, sitemap и абсолютные OG URLs.
При публикации задайте HTTP-заголовки CSP (как в meta страницы),
`X-Content-Type-Options: nosniff` и `Referrer-Policy: strict-origin-when-cross-origin`.

Сайт не обещает доступность неперенесённых твиков и не выдаёт иллюстрацию
за скриншот. Ссылка на IPA ведёт к инструкции, а не к несуществующему релизу.
Публичные Desktop-установщики пока не заявлены. Ссылки на GitHub — внешний сервис.

## Логотип и проверка

`assets/meganugget-logo.svg` генерируется из общего исходника:

```sh
python3 scripts/generate_branding.py
python3 -m unittest discover -s tests -p 'test_*.py' -v
```

См. `docs/BRANDING.md`. Авторский знак и оформление сайта относятся к MegaNugget,
не используют графику Apple или GoldenNugget.
