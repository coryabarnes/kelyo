// The first-run welcome: steps through a few screens over a twilight sky,
// with Kite's welcome tune playing. Written for Chrome's WebUI rules: no
// inline scripts and no innerHTML (Trusted Types).

import {WelcomeTune} from './music.js';

const tune = new WelcomeTune();
const steps = [...document.querySelectorAll('.step')];
const dots = [...document.querySelectorAll('.dot')];
const sound = document.getElementById('sound');
let current = 0;
let wantsMusic = true;

function show(index) {
  current = Math.max(0, Math.min(steps.length - 1, index));
  steps.forEach((step, i) => step.classList.toggle('active', i === current));
  dots.forEach((dot, i) => dot.classList.toggle('active', i === current));
  const focusable = steps[current].querySelector('.primary');
  if (focusable) {
    focusable.focus({preventScroll: true});
  }
}

function setSound(playing) {
  sound.setAttribute('aria-pressed', String(playing));
}

async function startMusic() {
  if (!wantsMusic) {
    return;
  }
  setSound(await tune.start());
}

// Try to start right away. If the browser wants a click first, the first
// click or key press anywhere starts it.
startMusic();
for (const event of ['pointerdown', 'keydown']) {
  window.addEventListener(event, startMusic, {once: true});
}

sound.addEventListener('click', event => {
  event.stopPropagation();
  wantsMusic = sound.getAttribute('aria-pressed') !== 'true';
  if (wantsMusic) {
    startMusic();
  } else {
    tune.stop(1.5);
    setSound(false);
  }
});

document.getElementById('begin').addEventListener('click', () => show(1));
for (const button of document.querySelectorAll('[data-next]')) {
  button.addEventListener('click', () => show(current + 1));
}
for (const dot of dots) {
  dot.addEventListener('click', () => show(Number(dot.dataset.step)));
}
window.addEventListener('keydown', event => {
  if (event.key === 'ArrowRight') {
    show(current + 1);
  } else if (event.key === 'ArrowLeft') {
    show(current - 1);
  }
});

// Let the music fade out gently before leaving.
document.getElementById('start').addEventListener('click', event => {
  event.preventDefault();
  const href = event.currentTarget.href;
  tune.stop(2);
  document.body.style.transition = 'opacity 1.6s ease';
  document.body.style.opacity = '0';
  setTimeout(() => {
    location.href = href;
  }, 1700);
});

// Stars: a few hundred points that twinkle slowly.
const canvas = document.getElementById('stars');
const context = canvas.getContext('2d');
let stars = [];

function resize() {
  const ratio = window.devicePixelRatio || 1;
  canvas.width = innerWidth * ratio;
  canvas.height = innerHeight * ratio;
  context.setTransform(ratio, 0, 0, ratio, 0, 0);
  const count = Math.round((innerWidth * innerHeight) / 5200);
  stars = Array.from({length: count}, () => ({
    x: Math.random() * innerWidth,
    y: Math.random() * innerHeight * 0.75,
    r: Math.random() * 1.1 + 0.2,
    phase: Math.random() * Math.PI * 2,
    speed: 0.3 + Math.random() * 0.9,
  }));
}

function draw(time) {
  context.clearRect(0, 0, innerWidth, innerHeight);
  for (const star of stars) {
    const fade = 1 - star.y / (innerHeight * 0.75);
    const twinkle = 0.55 + 0.45 * Math.sin(time / 1000 * star.speed + star.phase);
    context.globalAlpha = Math.max(0, fade * twinkle * 0.9);
    context.beginPath();
    context.arc(star.x, star.y, star.r, 0, Math.PI * 2);
    context.fillStyle = '#ffffff';
    context.fill();
  }
  requestAnimationFrame(draw);
}

resize();
window.addEventListener('resize', resize);
requestAnimationFrame(draw);
show(0);
