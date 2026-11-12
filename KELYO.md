# Kelyo Browser

> [!WARNING]
> **Kelyo is alpha software.** It is an early, personal project under active development: expect bugs, missing
> features, rough edges and breaking changes between versions. It is not ready for everyday use, and nothing here
> is a finished product. Try it for fun, and please report problems rather than relying on it.

The web. On your terms. Privacy by default. Intelligence by choice. Freedom by design.

Kelyo is built from [Ardia](https://github.com/ardia-browser/ardia) and its Linux port
(ardia-browser/ardia#622). Ardia's own source is Apache-2.0, and the browser as distributed is GPL-3.0 because its
engine carries Helium's patches; Kelyo keeps both licences, `LICENSE` and `NOTICE`.

What Kelyo changes:

- **Name and look:** Kelyo's name, logo, icons and wordmark throughout the interface (`apps/linux/kelyoBrand.ts`,
  `apps/linux/kelyo-assets`) and the engine (`engine/patches/build/kelyo-icons`, the branding patches and
  `ardia_namesub.py`). Pages show `kelyo://` addresses.
- **No calls home:** no usage statistics, remote switches or feedback uploads go to Ardia's servers; problem reports
  and the source link point at github.com/coryabarnes/kelyo.
- **Welcome tune:** an original generative ambient piece plays during the setup tour on Linux
  (`apps/linux/src/kelyoWelcomeTune.js`).
- **Themes:** eight gradient presets (Twilight, Aurora, Lagoon, Ember, Meadow, Peach, Fog, Ink) in the colour pickers,
  and Linux drawing for gradient swatches and the colour wheel.
- **Packaging:** a classic snap (`scripts/kelyo/pack-snap.sh`); data lives in `~/.config/kelyo-browser`.
  Saved passwords need `scripts/kelyo/dev.ardia.linux.policy` in `/usr/share/polkit-1/actions`.

Build: see `docs/linux-port.md` (engine) and `pnpm --filter @ardia/linux build` (interface).
