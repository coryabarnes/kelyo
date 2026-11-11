// Kelyo's built-in gradient themes, and the calls that read and set the theme.
// Shared by chrome://kelyo-theme and the welcome tour.

import {sendWithPromise} from 'chrome://resources/js/cr.js';

export const PRESETS = [
  {name: 'Twilight', colors: ['#2b1d5c', '#6b3f74', '#c27a6e']},
  {name: 'Aurora', colors: ['#0f3b4a', '#1f6f78', '#7ad3c2']},
  {name: 'Lagoon', colors: ['#0b2545', '#13315c', '#4f86c6']},
  {name: 'Ember', colors: ['#2a0f0f', '#7a2e1f', '#e07a3f']},
  {name: 'Meadow', colors: ['#1d3b2a', '#3f7d58', '#b7d48b']},
  {name: 'Peach', colors: ['#ffc3a0', '#ff9a9e', '#c3a6ff']},
  {name: 'Fog', colors: ['#eef0f5', '#dde3ec', '#c9d4e3']},
  {name: 'Ink', colors: ['#121218', '#24242e']},
];

export const DEFAULT_GRAIN = 0.35;

/** Resolves to {colors: string[], grain: number}. */
export function getTheme() {
  return sendWithPromise('getKelyoTheme');
}

/** Applies a theme to every Kelyo window. Empty `colors` resets to the default. */
export function setTheme(colors, grain) {
  chrome.send('setKelyoTheme', [colors, grain]);
}

/** A CSS gradient matching how Kelyo paints the sidebar. */
export function cssGradient(colors) {
  const stops = colors.length === 1 ? [colors[0], colors[0]] : colors;
  return `linear-gradient(170deg, ${stops.join(', ')})`;
}

/** A tile of noise as a data URL, for previews of the grain. */
export function grainTile() {
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 128;
  const context = canvas.getContext('2d');
  const image = context.createImageData(128, 128);
  for (let i = 0; i < image.data.length; i += 4) {
    const v = Math.random() * 255;
    image.data[i] = image.data[i + 1] = image.data[i + 2] = v;
    image.data[i + 3] = 255;
  }
  context.putImageData(image, 0, 0);
  return canvas.toDataURL();
}
