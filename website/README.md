# Swarx website

The public site for Swarx, built with [Tachyon](https://tachyon.del.ma) and
styled with [DuVay](https://duvay.del.ma). It is one static page with no
backend and no script of its own; DuVay's script handles the theme toggle.

## Build

The site uses `client/manifest.json` for its Tac configuration, so it needs a
Tachyon compiler newer than the 26.36.01 release. Use the same qualified
compiler that builds the app (`TACHYON_COMPILER`):

```sh
cd website
"$TACHYON_COMPILER" bundle
```

The output lands in `website/dist/web/`. Deploy that folder as static files.

## Layout

```
client/
  manifest.json      name, icons, page metadata, pinned DuVay sources
  pages/tac.html     the home page: components only
  components/
    site/            layout, header, footer and the shared section heading
    home/            hero, features, privacy and status sections
  shared/
    styles/site.css  Swarx colours over DuVay tokens, and body defaults
    assets/          the Swarx mark, app icons and vendored DuVay 26.35.01
```

Every claim on the page comes from the project README and in-app help. Keep
it that way: the page says what Swarx does today, not what is planned.
