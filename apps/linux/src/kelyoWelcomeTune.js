
const CHORD_SECONDS = 8;
const CHORDS = [
  [50, 57, 64, 66, 73],
  [47, 54, 62, 64, 69],
  [43, 50, 59, 66, 73],
  [45, 52, 64, 71, 73],
];
const CHORD_TINT = [76, 74, 73, 71];

const PENTATONIC = [62, 64, 66, 69, 71, 74, 76, 78, 81, 83, 86];

function frequency(midi) {
  return 440 * Math.pow(2, (midi - 69) / 12);
}
function seeded(seed) {
  return function next() {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
function makeImpulse(context, seconds, decay) {
  const rate = context.sampleRate;
  const length = Math.floor(rate * seconds);
  const impulse = context.createBuffer(2, length, rate);
  const random = seeded(7);
  for (let channel = 0; channel < 2; channel++) {
    const data = impulse.getChannelData(channel);
    let last = 0;
    for (let i = 0; i < length; i++) {
      const t = i / length;
      const white = random() * 2 - 1;
      last = last + (white - last) * (0.55 - 0.35 * t);
      data[i] = last * Math.pow(1 - t, decay);
    }
  }
  return impulse;
}

export class WelcomeTune {
  constructor() {
    this.context = null;
    this.master = null;
    this.timer = null;
    this.nextChordTime = 0;
    this.chordIndex = 0;
    this.random = seeded(2026);
    this.playing = false;
  }

    async start() {
    if (!this.context) {
      this.build();
    }
    await Promise.race([
      this.context.resume(),
      new Promise(resolve => setTimeout(resolve, 400)),
    ]);
    if (this.context.state !== 'running') {
      this.context.close();
      this.context = null;
      return false;
    }
    if (!this.playing) {
      this.playing = true;
      const now = this.context.currentTime;
      this.master.gain.cancelScheduledValues(now);
      this.master.gain.setValueAtTime(this.master.gain.value, now);
      this.master.gain.linearRampToValueAtTime(0.75, now + 4);
      this.nextChordTime = Math.max(this.nextChordTime, now + 0.1);
      this.schedule();
      this.timer = setInterval(() => this.schedule(), 500);
    }
    return true;
  }

    stop(seconds = 3) {
    if (!this.context || !this.playing) {
      return;
    }
    this.playing = false;
    clearInterval(this.timer);
    const now = this.context.currentTime;
    this.master.gain.cancelScheduledValues(now);
    this.master.gain.setValueAtTime(this.master.gain.value, now);
    this.master.gain.linearRampToValueAtTime(0, now + seconds);
    setTimeout(() => {
      if (!this.playing) {
        this.context.suspend();
      }
    }, seconds * 1000 + 8000);
  }

    static render(seconds, sampleRate = 44100) {
    const tune = new WelcomeTune();
    tune.build(new OfflineAudioContext(2, Math.ceil(seconds * sampleRate), sampleRate));
    tune.master.gain.setValueAtTime(0, 0);
    tune.master.gain.linearRampToValueAtTime(0.75, 4);
    while (tune.nextChordTime < seconds) {
      tune.playChord(tune.chordIndex % CHORDS.length, tune.nextChordTime);
      tune.nextChordTime += CHORD_SECONDS;
      tune.chordIndex += 1;
    }
    return tune.context.startRendering();
  }

  build(context = new AudioContext()) {
    this.context = context;

    this.master = context.createGain();
    this.master.gain.value = 0;
    const compressor = context.createDynamicsCompressor();
    compressor.threshold.value = -18;
    compressor.ratio.value = 3;
    this.master.connect(compressor).connect(context.destination);
    this.reverb = context.createConvolver();
    this.reverb.buffer = makeImpulse(context, 8, 2.6);
    const reverbLevel = context.createGain();
    reverbLevel.gain.value = 0.9;
    this.reverb.connect(reverbLevel).connect(this.master);
    const left = context.createDelay(2);
    const right = context.createDelay(2);
    left.delayTime.value = 0.75;
    right.delayTime.value = 0.75;
    const feedback = context.createGain();
    feedback.gain.value = 0.42;
    const darken = context.createBiquadFilter();
    darken.type = 'lowpass';
    darken.frequency.value = 3200;
    const merger = context.createChannelMerger(2);
    this.echo = context.createGain();
    this.echo.connect(left);
    left.connect(darken).connect(right);
    right.connect(feedback).connect(left);
    left.connect(merger, 0, 0);
    right.connect(merger, 0, 1);
    const echoLevel = context.createGain();
    echoLevel.gain.value = 0.35;
    merger.connect(echoLevel);
    echoLevel.connect(this.master);
    echoLevel.connect(this.reverb);
    this.padFilter = context.createBiquadFilter();
    this.padFilter.type = 'lowpass';
    this.padFilter.frequency.value = 1100;
    this.padFilter.Q.value = 0.7;
    const sweep = context.createOscillator();
    sweep.frequency.value = 1 / 13;
    const sweepDepth = context.createGain();
    sweepDepth.gain.value = 650;
    sweep.connect(sweepDepth).connect(this.padFilter.frequency);
    sweep.start();
    const padDry = context.createGain();
    padDry.gain.value = 0.5;
    this.padFilter.connect(padDry).connect(this.master);
    this.padFilter.connect(this.reverb);
  }

    schedule() {
    const horizon = this.context.currentTime + 2;
    while (this.nextChordTime < horizon) {
      this.playChord(this.chordIndex % CHORDS.length, this.nextChordTime);
      this.nextChordTime += CHORD_SECONDS;
      this.chordIndex += 1;
    }
  }

  playChord(index, time) {
    const notes = CHORDS[index];
    notes.forEach((note, voice) => {
      this.pad(note, time + voice * 0.35, CHORD_SECONDS + 3, voice === 0 ? 0.11 : 0.07);
    });
    this.shimmer(CHORD_TINT[index] + 12, time + 1.5, CHORD_SECONDS);
    const count = 4 + Math.floor(this.random() * 3);
    let at = time + 0.8 + this.random();
    for (let i = 0; i < count && at < time + CHORD_SECONDS - 0.4; i++) {
      const note = this.random() < 0.3
        ? CHORD_TINT[index]
        : PENTATONIC[Math.floor(this.random() * PENTATONIC.length)];
      this.bell(note, at, 0.05 + this.random() * 0.03);
      at += 0.5 * (1 + Math.floor(this.random() * 3));
    }
  }

  pad(note, time, length, level) {
    const context = this.context;
    const envelope = context.createGain();
    envelope.gain.setValueAtTime(0, time);
    envelope.gain.linearRampToValueAtTime(level, time + 3.2);
    envelope.gain.setValueAtTime(level, time + length - 4);
    envelope.gain.linearRampToValueAtTime(0, time + length);
    const panner = context.createStereoPanner();
    panner.pan.value = ((note * 7) % 11) / 11 - 0.5;
    envelope.connect(panner).connect(this.padFilter);

    const base = frequency(note);
    const partials = [
      ['sine', 1, 0, 1],
      ['triangle', 1, 7, 0.45],
      ['triangle', 1, -6, 0.45],
      ['sine', 2, 3, 0.18],
    ];
    for (const [type, ratio, cents, gain] of partials) {
      const oscillator = context.createOscillator();
      oscillator.type = type;
      oscillator.frequency.value = base * ratio;
      oscillator.detune.value = cents;
      const partial = context.createGain();
      partial.gain.value = gain;
      oscillator.connect(partial).connect(envelope);
      oscillator.start(time);
      oscillator.stop(time + length + 0.1);
    }
  }

  bell(note, time, level) {
    const context = this.context;
    const base = frequency(note);
    const out = context.createGain();
    out.gain.value = 1;
    const panner = context.createStereoPanner();
    panner.pan.value = this.random() * 1.2 - 0.6;
    out.connect(panner);
    panner.connect(this.master);
    panner.connect(this.echo);
    panner.connect(this.reverb);
    const partials = [[1, 1, 3.5], [2.76, 0.32, 1.6], [5.4, 0.12, 0.8], [8.93, 0.05, 0.4]];
    for (const [ratio, gain, decay] of partials) {
      const oscillator = context.createOscillator();
      oscillator.type = 'sine';
      oscillator.frequency.value = base * ratio;
      const envelope = context.createGain();
      envelope.gain.setValueAtTime(0, time);
      envelope.gain.linearRampToValueAtTime(level * gain, time + 0.008);
      envelope.gain.exponentialRampToValueAtTime(0.0001, time + decay);
      oscillator.connect(envelope).connect(out);
      oscillator.start(time);
      oscillator.stop(time + decay + 0.05);
    }
  }

  shimmer(note, time, length) {
    const context = this.context;
    const oscillator = context.createOscillator();
    oscillator.type = 'sine';
    oscillator.frequency.value = frequency(note);
    const tremolo = context.createOscillator();
    tremolo.frequency.value = 4.5;
    const tremoloDepth = context.createGain();
    tremoloDepth.gain.value = 0.006;
    const envelope = context.createGain();
    envelope.gain.setValueAtTime(0, time);
    envelope.gain.linearRampToValueAtTime(0.012, time + length / 2);
    envelope.gain.linearRampToValueAtTime(0, time + length);
    tremolo.connect(tremoloDepth).connect(envelope.gain);
    oscillator.connect(envelope);
    envelope.connect(this.reverb);
    oscillator.start(time);
    tremolo.start(time);
    oscillator.stop(time + length + 0.1);
    tremolo.stop(time + length + 0.1);
  }
}
