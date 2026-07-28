(() => {
  "use strict";

  const stems = [
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
  const Context = window.AudioContext || window.webkitAudioContext;
  const canLoadAudio = Boolean(Context && typeof window.fetch === "function");
  let encoded = null;
  const buffers = new Map();
  const activeSources = new Set();
  let context = null;
  let master = null;
  let decodeStarted = false;
  let decodeComplete = false;
  let failed = !canLoadAudio;
  let masterVolume = 0.7;

  function prepareBank() {
    if (encoded || !canLoadAudio) return;
    encoded = new Map(
      stems.map((stem) => [
        stem,
        window
          .fetch(new URL(`audio/${stem}.wav`, document.baseURI))
          .then((response) => {
            if (!response.ok) throw new Error("audio asset unavailable");
            return response.arrayBuffer();
          })
          .catch(() => null),
      ]),
    );
  }

  function ensureContext() {
    if (failed) return false;
    if (context) return true;
    try {
      prepareBank();
      context = new Context({ latencyHint: "interactive" });
      master = context.createGain();
      master.gain.value = masterVolume;
      master.connect(context.destination);
      decodeBank();
      return true;
    } catch (_error) {
      failed = true;
      context = null;
      master = null;
      return false;
    }
  }

  async function decodeBank() {
    if (decodeStarted || !context) return;
    decodeStarted = true;
    prepareBank();
    const results = await Promise.all(
      [...(encoded || [])].map(async ([stem, pending]) => {
        const bytes = await pending;
        if (!bytes) return;
        try {
          const buffer = await context.decodeAudioData(bytes.slice(0));
          buffers.set(stem, buffer);
        } catch (_error) {
          // A missing effect never prevents the rest of the bank from playing.
        }
      }),
    );
    decodeComplete = results.length === stems.length;
  }

  function activate() {
    if (!ensureContext()) return false;
    if (context.state !== "running") {
      context.resume().catch(() => {});
    }
    return true;
  }

  function stopAll() {
    for (const source of activeSources) {
      try {
        source.stop();
      } catch (_error) {
        // A source may already have stopped between iteration and this call.
      }
    }
    activeSources.clear();
  }

  function suspend() {
    stopAll();
    if (context?.state === "running") {
      context.suspend().catch(() => {});
    }
  }

  window.oxidefallAudioAvailable = () => !failed;
  window.oxidefallAudioPrepare = prepareBank;
  window.oxidefallAudioActivate = activate;
  window.oxidefallAudioSetMasterVolume = (volume) => {
    masterVolume = Math.max(0, Math.min(1, Number(volume) || 0));
    if (master && context) {
      master.gain.setTargetAtTime(masterVolume, context.currentTime, 0.015);
    }
  };
  window.oxidefallAudioPlay = (name, gainDb, rate, pan, delaySeconds) => {
    if (!activate()) return;
    const buffer = buffers.get(name);
    if (!buffer || !master || masterVolume <= 0) return;

    while (activeSources.size >= 16) {
      const oldest = activeSources.values().next().value;
      try {
        oldest.stop();
      } catch (_error) {
        // It is safe to discard an already-ended source.
      }
      activeSources.delete(oldest);
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
      panner.connect(master);
    } else {
      gain.connect(master);
    }

    source.onended = () => activeSources.delete(source);
    activeSources.add(source);
    source.start(context.currentTime + Math.max(0, Number(delaySeconds) || 0));
  };
  window.oxidefallAudioStopAll = stopAll;
  window.oxidefallAudioDebugState = () => ({
    available: !failed,
    ready: decodeComplete && buffers.size === stems.length,
    activeVoices: activeSources.size,
    contextState: context?.state || "uninitialized",
    masterVolume,
  });

  const unlock = () => activate();
  window.addEventListener("pointerdown", unlock, { capture: true });
  window.addEventListener("keydown", unlock, { capture: true });
  window.addEventListener("blur", suspend);
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) suspend();
  });
})();
