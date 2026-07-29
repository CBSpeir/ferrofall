(() => {
  "use strict";

  const effectStems = [
    "ui_activate",
    "game_start",
    "pause",
    "resume",
    "move_a",
    "move_b",
    "rotate",
    "hold",
    "contact",
    "hard_drop",
    "lock",
    "clear_single",
    "clear_double",
    "clear_triple",
    "clear_four",
    "accent_tspin",
    "accent_combo",
    "accent_back_to_back",
    "accent_perfect",
    "level_up",
    "game_over",
    "new_best",
  ];
  const musicStems = ["music_base", "music_drive", "music_pressure"];
  const musicBarSeconds = 240 / 132;
  const Context = window.AudioContext || window.webkitAudioContext;
  const canLoadAudio = Boolean(Context && typeof window.fetch === "function");

  const effectBuffers = new Map();
  const musicBuffers = new Map();
  const activeEffects = new Set();
  let musicSources = Array(musicStems.length).fill(null);
  let musicLayerGains = Array(musicStems.length).fill(null);
  const musicRequests = new Map();
  let context = null;
  let master = null;
  let effectsBus = null;
  let musicBus = null;
  let musicDuck = null;
  let effectsRequest = null;
  let effectsReady = false;
  let startupCuePlayed = false;
  let failed = !canLoadAudio;
  let musicFailed = !canLoadAudio;
  let muted = false;
  let effectsVolume = 0.49;
  let musicVolume = 0.1225;
  let musicRequested = false;
  let musicPaused = false;
  let musicTier = 1;
  let musicClockOffset = 0;
  let musicClockAnchor = 0;
  let musicClockRunning = false;

  function fetchAsset(path) {
    return window
      .fetch(new URL(path, document.baseURI))
      .then((response) => {
        if (!response.ok) throw new Error("audio asset unavailable");
        return response.arrayBuffer();
      })
      .catch(() => null);
  }

  function ensureContext() {
    if (failed) return false;
    if (context) return true;
    try {
      context = new Context({ latencyHint: "interactive" });
      master = context.createGain();
      effectsBus = context.createGain();
      musicBus = context.createGain();
      musicDuck = context.createGain();
      master.gain.value = muted ? 0 : 1;
      effectsBus.gain.value = effectsVolume;
      musicBus.gain.value = musicVolume;
      musicDuck.gain.value = 1;
      effectsBus.connect(master);
      musicBus.connect(musicDuck);
      musicDuck.connect(master);
      master.connect(context.destination);
      return true;
    } catch (_error) {
      failed = true;
      musicFailed = true;
      context = null;
      master = null;
      effectsBus = null;
      musicBus = null;
      musicDuck = null;
      return false;
    }
  }

  function loadEffects() {
    if (effectsRequest || !context) return effectsRequest;
    effectsRequest = Promise.all(
      effectStems.map(async (stem) => {
        const bytes = await fetchAsset(`audio/${stem}.ogg`);
        if (!bytes || !context) return;
        try {
          effectBuffers.set(stem, await context.decodeAudioData(bytes.slice(0)));
        } catch (_error) {
          // Missing assets are isolated from the rest of the bank.
        }
      }),
    ).then(() => {
      effectsReady = effectBuffers.size === effectStems.length;
    });
    return effectsRequest;
  }

  function loadMusicStem(index) {
    const stem = musicStems[index];
    if (!context || !stem) return Promise.resolve();
    if (musicBuffers.has(stem)) return Promise.resolve();
    if (musicRequests.has(stem)) return musicRequests.get(stem);
    const request = fetchAsset(`audio/${stem}.ogg`)
      .then(async (bytes) => {
        if (!bytes || !context) throw new Error("music asset unavailable");
        musicBuffers.set(stem, await context.decodeAudioData(bytes.slice(0)));
        if (musicRequested && !musicPaused) startLoadedMusicStem(index);
      })
      .catch(() => {
        if (index === 0) musicFailed = true;
      });
    musicRequests.set(stem, request);
    return request;
  }

  function activate() {
    if (!ensureContext()) return false;
    if (context.state !== "running") {
      context.resume().catch(() => {});
    }
    loadEffects();
    return true;
  }

  function stopEffects() {
    for (const source of activeEffects) {
      try {
        source.stop();
      } catch (_error) {
        // A source may already have stopped between iteration and this call.
      }
    }
    activeEffects.clear();
  }

  function stopMusicSources() {
    for (const source of musicSources) {
      if (!source) continue;
      try {
        source.stop();
      } catch (_error) {
        // A source may already have stopped between iteration and this call.
      }
    }
    musicSources = Array(musicStems.length).fill(null);
    musicLayerGains = Array(musicStems.length).fill(null);
  }

  function musicDuration() {
    return musicBuffers.get(musicStems[0])?.duration || 0;
  }

  function normalizeMusicPosition(position) {
    const duration = musicDuration();
    if (duration <= 0) return 0;
    return ((position % duration) + duration) % duration;
  }

  function currentMusicPosition() {
    if (!context || !musicClockRunning) return musicClockOffset;
    return normalizeMusicPosition(
      musicClockOffset + Math.max(0, context.currentTime - musicClockAnchor),
    );
  }

  function delayToNextBar(position) {
    const remainder = ((position % musicBarSeconds) + musicBarSeconds) % musicBarSeconds;
    if (remainder <= 0.02 || musicBarSeconds - remainder <= 0.02) return 0;
    return musicBarSeconds - remainder;
  }

  function layerGain(index, tier = musicTier) {
    return index < tier ? 1 : 0;
  }

  function startMusicSource(index, offset, when) {
    if (!context || !musicBus || musicSources[index]) return;
    const buffer = musicBuffers.get(musicStems[index]);
    if (!buffer) return;
    const source = context.createBufferSource();
    const gain = context.createGain();
    const panner =
      typeof context.createStereoPanner === "function"
        ? context.createStereoPanner()
        : null;
    source.buffer = buffer;
    source.loop = true;
    source.loopStart = 0;
    source.loopEnd = buffer.duration;
    gain.gain.value = layerGain(index);
    source.connect(gain);
    if (panner) {
      panner.pan.value = index === 1 ? -0.08 : index === 2 ? 0.08 : 0;
      gain.connect(panner);
      panner.connect(musicBus);
    } else {
      gain.connect(musicBus);
    }
    source.start(when, normalizeMusicPosition(offset));
    musicSources[index] = source;
    musicLayerGains[index] = gain;
  }

  function startMusicSources(offset, when) {
    musicStems.forEach((_stem, index) => startMusicSource(index, offset, when));
  }

  function startLoadedMusicStem(index) {
    if (!context || musicPaused || !musicRequested || musicSources[index]) return;
    const position = currentMusicPosition();
    const delay = index === 0 ? 0.02 : delayToNextBar(position);
    startMusicSource(index, position + delay, context.currentTime + delay);
  }

  function setMusicTier(tier) {
    musicTier = Math.max(1, Math.min(3, Math.round(Number(tier) || 1)));
    if (!context || musicLayerGains.length === 0) return;
    const delay = delayToNextBar(currentMusicPosition());
    const start = context.currentTime + delay;
    for (let index = 0; index < musicLayerGains.length; index += 1) {
      const parameter = musicLayerGains[index]?.gain;
      if (!parameter) continue;
      parameter.cancelScheduledValues(context.currentTime);
      parameter.setValueAtTime(parameter.value, context.currentTime);
      parameter.setValueAtTime(parameter.value, start);
      parameter.linearRampToValueAtTime(layerGain(index), start + 0.25);
    }
    for (let index = 0; index < musicTier; index += 1) {
      loadMusicStem(index);
    }
  }

  function startMusic(tier) {
    if (!activate() || !context) return;
    stopMusicSources();
    musicRequested = true;
    musicPaused = false;
    musicTier = Math.max(1, Math.min(3, Math.round(Number(tier) || 1)));
    musicClockOffset = 0;
    const startAt = context.currentTime + 0.06;
    musicClockAnchor = startAt;
    musicClockRunning = true;
    loadMusicStem(0);
    setTimeout(() => loadMusicStem(1), 15000);
    setTimeout(() => loadMusicStem(2), 45000);
  }

  function pauseMusic() {
    if (!context || !musicRequested || musicPaused) return;
    musicClockOffset = currentMusicPosition();
    musicClockRunning = false;
    musicPaused = true;
    stopMusicSources();
  }

  function resumeMusic() {
    if (!activate() || !context || !musicRequested || !musicPaused) return;
    musicPaused = false;
    const startAt = context.currentTime + 0.04;
    musicClockAnchor = startAt;
    musicClockRunning = true;
    if (musicBuffers.size > 0) {
      startMusicSources(musicClockOffset, startAt);
    }
  }

  function stopMusic() {
    stopMusicSources();
    musicRequested = false;
    musicPaused = false;
    musicClockRunning = false;
    musicClockOffset = 0;
  }

  function suspend() {
    stopEffects();
    pauseMusic();
    if (context?.state === "running") {
      context.suspend().catch(() => {});
    }
  }

  window.oxidefallAudioAvailable = () => !failed;
  window.oxidefallMusicAvailable = () => !failed && !musicFailed;
  window.oxidefallAudioPrepare = () => {};
  window.oxidefallAudioActivate = activate;
  window.oxidefallAudioSetMuted = (nextMuted) => {
    muted = Boolean(nextMuted);
    if (master && context) {
      master.gain.setTargetAtTime(muted ? 0 : 1, context.currentTime, 0.015);
    }
  };
  window.oxidefallAudioSetEffectsVolume = (volume) => {
    effectsVolume = Math.max(0, Math.min(1, Number(volume) || 0));
    if (effectsBus && context) {
      effectsBus.gain.setTargetAtTime(effectsVolume, context.currentTime, 0.015);
    }
  };
  window.oxidefallAudioSetMusicVolume = (volume) => {
    musicVolume = Math.max(0, Math.min(1, Number(volume) || 0));
    if (musicBus && context) {
      musicBus.gain.setTargetAtTime(musicVolume, context.currentTime, 0.015);
    }
  };
  window.oxidefallAudioPlay = (name, gainDb, rate, pan, delaySeconds) => {
    if (!activate()) return;
    const buffer = effectBuffers.get(name);
    if (!buffer || !effectsBus || effectsVolume <= 0) {
      if (!startupCuePlayed && effectsBus && effectsVolume > 0) {
        const oscillator = context.createOscillator();
        const gain = context.createGain();
        oscillator.type = "square";
        oscillator.frequency.value = 1080;
        gain.gain.setValueAtTime(0.08, context.currentTime);
        gain.gain.exponentialRampToValueAtTime(0.001, context.currentTime + 0.055);
        oscillator.connect(gain);
        gain.connect(effectsBus);
        oscillator.start();
        oscillator.stop(context.currentTime + 0.055);
        startupCuePlayed = true;
      }
      return;
    }

    while (activeEffects.size >= 16) {
      const oldest = activeEffects.values().next().value;
      try {
        oldest.stop();
      } catch (_error) {
        // It is safe to discard an already-ended source.
      }
      activeEffects.delete(oldest);
    }

    const source = context.createBufferSource();
    const gain = context.createGain();
    source.buffer = buffer;
    source.playbackRate.value = Math.max(0.5, Math.min(2, Number(rate) || 1));
    gain.gain.value = 10 ** (Math.min(0, Number(gainDb) || 0) / 20);
    source.connect(gain);

    if (typeof context.createStereoPanner === "function") {
      const panner = context.createStereoPanner();
      panner.pan.value = Math.max(-0.35, Math.min(0.35, Number(pan) || 0));
      gain.connect(panner);
      panner.connect(effectsBus);
    } else {
      gain.connect(effectsBus);
    }

    source.onended = () => activeEffects.delete(source);
    activeEffects.add(source);
    source.start(context.currentTime + Math.max(0, Number(delaySeconds) || 0));
  };
  window.oxidefallAudioStopEffects = stopEffects;
  window.oxidefallAudioStopAll = () => {
    stopEffects();
    stopMusic();
  };
  window.oxidefallMusicStart = startMusic;
  window.oxidefallMusicSetTier = setMusicTier;
  window.oxidefallMusicPause = pauseMusic;
  window.oxidefallMusicResume = resumeMusic;
  window.oxidefallMusicStop = stopMusic;
  window.oxidefallMusicSetDucked = (ducked) => {
    if (!musicDuck || !context) return;
    musicDuck.gain.setTargetAtTime(
      ducked ? 0.70794576 : 1,
      context.currentTime,
      ducked ? 0.008 : 0.06,
    );
  };
  window.oxidefallAudioDebugState = () => {
    const activeMusicSources = musicSources.filter(Boolean).length;
    return {
      available: !failed,
      ready: effectsReady,
      effectsRequested: effectsRequest !== null,
      startupCuePlayed,
      activeVoices: activeEffects.size + activeMusicSources,
      contextState: context?.state || "uninitialized",
      muted,
      effectsVolume,
      musicVolume,
      musicAvailable: !failed && !musicFailed,
      musicReady: musicBuffers.has(musicStems[0]),
      musicPlaying: Boolean(musicSources[0]),
      loadedMusicStems: musicStems.filter((stem) => musicBuffers.has(stem)),
      requestedMusicStems: [...musicRequests.keys()],
      musicPaused,
      musicTier,
      musicPosition: currentMusicPosition(),
    };
  };

  const unlock = () => activate();
  window.addEventListener("pointerdown", unlock, { capture: true });
  window.addEventListener("keydown", unlock, { capture: true });
  window.addEventListener("blur", suspend);
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) suspend();
  });
})();
