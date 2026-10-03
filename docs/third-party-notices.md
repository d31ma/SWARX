# Third-party notices

These notices describe the files inspected in this source tree. They do not
select a license for Swarx application code. Preserve each dependency's full
license text and any notice in its distributed asset when redistributing it.

| Dependency | Inspected version and files | Existing notice |
| --- | --- | --- |
| DuVay | `26.35.01`; bundled CSS, JavaScript and web-component JavaScript in `app/client/shared/assets/duvay/` | MIT; copyright 2026 DELMA. Full [license](../app/client/shared/assets/duvay/LICENSE) and [version record](../app/client/shared/assets/duvay/VERSION) are retained. |
| Chart.js | `4.5.1`, from the bundled `chart.umd.min.js` header | The bundle says MIT and copyright 2025 Chart.js Contributors. Its adjacent [license](../app/client/shared/assets/chartjs/LICENSE.md) says copyright 2014-2024 Chart.js Contributors. The retained license matches the copyright and wording in the [upstream v4.5.1 license](https://raw.githubusercontent.com/chartjs/Chart.js/v4.5.1/LICENSE.md). Preserve both upstream notices; neither original is rewritten. |
| Tachyon declarations | `app/tachyon-env.d.ts` and `app/tachyon-behaviors.d.ts` exactly match the compiler scaffold templates at framework commit `e1a3996a850f884594e6f9d6e2e4a767dfe15ab0` | MIT; copyright 2026 Chidelma. The unchanged full [license](licenses/tachyon-LICENSE.txt) is retained from the matching framework revision. |
| Tailwind CSS | `4.3.3`, from generated CSS banners | MIT. The unchanged full [license](licenses/tailwindcss-LICENSE.txt), copyright Tailwind Labs, Inc., was recovered from an installed `tailwindcss` package whose metadata reports version `4.3.3`. |

The inspected DuVay styles name IBM Plex Sans and system font fallbacks. No
font binary or `@font-face` rule was found in the source tree or inspected
bundled styles. Naming an installed font does not add a redistributed font
asset. Any future bundled font needs its own source and license review.

The Tachyon environment and behavior declarations are scaffold source files,
with exact matching template provenance recorded above. Their retained license
does not turn a compiler binary or framework runtime into part of this source
inventory. No compiler or runtime distribution is prepared here.

Rust dependencies and development-only packages are separate from these bundled
browser assets. Their transitive notices must be checked against the actual
resolved dependencies for any future binary distribution. The authentic
`app/package-lock.json` now records TypeScript7.0.2 and its optional platform
packages; lock provenance and frozen dry-runs do not complete a future binary
or transitive-license audit. See [setup](setup.md) for its verified scope.

No project-wide application license was invented. Public hosting policy and any
license choice remain separate from preserving mandatory third-party notices.
