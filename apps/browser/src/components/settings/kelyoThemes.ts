import { padPosition, type ProfileTheme, type ThemeColor } from "../../lib/profileLook";

const channel = (hex: string, at: number) => parseInt(hex.slice(at, at + 2), 16) / 255;
const fromHex = (hex: string): ThemeColor => ({ r: channel(hex, 1), g: channel(hex, 3), b: channel(hex, 5), a: 1 });

const gradient = (colors: string[]): ProfileTheme => ({
  stops: colors.map((hex) => {
    const color = fromHex(hex);
    return { color, ...padPosition(color) };
  }),
  intensity: 1,
  mode: "auto",
  grain: 0.35,
  texture: "grain",
  translucency: 0,
});

export type KelyoTheme = { name: string; colors: string[]; theme: ProfileTheme };

export const KELYO_THEMES: KelyoTheme[] = [
  { name: "Twilight", colors: ["#2b1d5c", "#6b3f74", "#c27a6e"] },
  { name: "Aurora", colors: ["#0f3b4a", "#1f6f78", "#7ad3c2"] },
  { name: "Lagoon", colors: ["#0b2545", "#13315c", "#4f86c6"] },
  { name: "Ember", colors: ["#2a0f0f", "#7a2e1f", "#e07a3f"] },
  { name: "Meadow", colors: ["#1d3b2a", "#3f7d58", "#b7d48b"] },
  { name: "Peach", colors: ["#ffc3a0", "#ff9a9e", "#c3a6ff"] },
  { name: "Fog", colors: ["#eef0f5", "#dde3ec", "#c9d4e3"] },
  { name: "Ink", colors: ["#121218", "#24242e"] },
].map((preset) => ({ ...preset, theme: gradient(preset.colors) }));

const sameColor = (a: ThemeColor, b: ThemeColor) => Math.abs(a.r - b.r) + Math.abs(a.g - b.g) + Math.abs(a.b - b.b) < 0.02;

export const isKelyoTheme = (custom: ProfileTheme | null | undefined, preset: KelyoTheme) =>
  !!custom && custom.stops.length === preset.theme.stops.length && custom.stops.every((stop, i) => sameColor(stop.color, preset.theme.stops[i]!.color));
