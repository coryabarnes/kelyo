// chrome://kelyo-theme: choose a preset or up to three colours, plus grain.
// Every change is applied to all Kelyo windows right away.

import {cssGradient, DEFAULT_GRAIN, getTheme, grainTile, PRESETS, setTheme} from './themes.js';

const presetsEl = document.getElementById('presets');
const wellsEl = document.getElementById('wells');
const grainEl = document.getElementById('grain');
const addEl = document.getElementById('add-color');
const removeEl = document.getElementById('remove-color');
const sidebarEl = document.getElementById('preview-sidebar');
const grainOverlayEl = document.getElementById('preview-grain');

let colors = [];
let grain = DEFAULT_GRAIN;
let applyTimer = 0;

grainOverlayEl.style.backgroundImage = `url(${grainTile()})`;

function sameColors(a, b) {
  return a.length === b.length && a.every((color, i) => color === b[i]);
}

function render() {
  sidebarEl.style.background = colors.length ? cssGradient(colors) : '';
  grainOverlayEl.style.opacity = colors.length ? String(grain * 0.6) : '0';
  for (const button of presetsEl.children) {
    button.setAttribute('aria-pressed',
        String(sameColors(PRESETS[Number(button.dataset.index)].colors, colors)));
  }
  wellsEl.replaceChildren(...colors.map((color, i) => {
    const input = document.createElement('input');
    input.type = 'color';
    input.value = color;
    input.setAttribute('aria-label', `Colour ${i + 1}`);
    input.addEventListener('input', () => {
      colors[i] = input.value;
      changed(/*rerender=*/false);
    });
    return input;
  }));
  addEl.disabled = colors.length >= 3;
  removeEl.disabled = colors.length <= 1;
  grainEl.value = String(grain);
}

// Repaints the preview now and the browser shortly after (colour pickers fire
// many events while dragging).
function changed(rerender = true) {
  if (rerender) {
    render();
  } else {
    sidebarEl.style.background = cssGradient(colors);
  }
  clearTimeout(applyTimer);
  applyTimer = setTimeout(() => setTheme(colors, grain), 120);
}

PRESETS.forEach((preset, index) => {
  const button = document.createElement('button');
  button.className = 'preset';
  button.dataset.index = String(index);
  const swatch = document.createElement('span');
  swatch.className = 'swatch';
  swatch.style.background = cssGradient(preset.colors);
  const name = document.createElement('span');
  name.textContent = preset.name;
  button.append(swatch, name);
  button.addEventListener('click', () => {
    colors = [...preset.colors];
    changed();
  });
  presetsEl.append(button);
});

addEl.addEventListener('click', () => {
  if (colors.length === 0) {
    colors = ['#2b1d5c'];
  } else if (colors.length < 3) {
    colors.push(colors[colors.length - 1]);
  }
  changed();
});

removeEl.addEventListener('click', () => {
  colors.pop();
  changed();
});

grainEl.addEventListener('input', () => {
  grain = Number(grainEl.value);
  grainOverlayEl.style.opacity = String(grain * 0.6);
  clearTimeout(applyTimer);
  applyTimer = setTimeout(() => setTheme(colors, grain), 120);
});

document.getElementById('reset').addEventListener('click', () => {
  colors = [];
  grain = DEFAULT_GRAIN;
  changed();
});

getTheme().then(theme => {
  colors = theme.colors;
  grain = theme.grain;
  render();
});
render();
