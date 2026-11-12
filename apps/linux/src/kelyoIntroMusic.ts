import { WelcomeTune } from "./kelyoWelcomeTune.js";

const GESTURES = ["pointerdown", "keydown"] as const;

const tune = new WelcomeTune();
let wanted = false;
let muted = false;

async function startIfWanted(): Promise<void> {
  if (!wanted || muted) return;
  if (await tune.start()) listenForGesture(false);
}

const onGesture = () => void startIfWanted();

function listenForGesture(on: boolean): void {
  for (const type of GESTURES) {
    if (on) window.addEventListener(type, onGesture, true);
    else window.removeEventListener(type, onGesture, true);
  }
}

export function playIntroMusic(_cues: unknown, isMuted: boolean): Promise<void> {
  wanted = true;
  muted = isMuted;
  listenForGesture(true);
  return startIfWanted();
}

export function setIntroMusicMuted(isMuted: boolean): Promise<void> {
  muted = isMuted;
  if (muted) {
    tune.stop(0.6);
    return Promise.resolve();
  }
  listenForGesture(true);
  return startIfWanted();
}

export function stopIntroMusic(fade = 0.4): Promise<void> {
  wanted = false;
  listenForGesture(false);
  tune.stop(fade);
  return Promise.resolve();
}
