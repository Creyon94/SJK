// Sol JK site: WebGPU effects from the MIT-licensed Shaders library
// (https://github.com/shader-effects-inc/shaders, vendored as
// vendor/shaders/shaders-4.0.0.js so no third-party server is contacted, with its
// telemetry off). The hero gets golden god rays, a slow sunburst behind the
// emblem, drifting dust and a lightsaber trail that follows the pointer. When the
// visitor asks for reduced motion (Windows' "Animation effects" off does), the
// same scene is drawn once and held still, without the trail. Without WebGPU
// the script does nothing and the CSS starfield stays.

const LIBRARY = "./vendor/shaders/shaders-4.0.0.js";
/** The library's `onError` reasons after which a shader never draws again. */
const TERMINAL = new Set([
  "unsupported", "no-adapter", "no-device", "init-failed", "device-lost", "out-of-memory",
  "gpu-error", "render-failed", "limit-exceeded", "unrecoverable", "rebuild_failed",
]);

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const coarsePointer = window.matchMedia("(pointer: coarse)").matches;
/** A speed, or none when the scene is held still. */
const pace = (speed) => (reducedMotion ? 0 : speed);
/** How long a still scene runs before it is paused, so it has drawn a frame. */
const STILL_AFTER_MS = 400;

/** The hero's layers, bottom to top, in the site's gold and the client's saber blue. */
function heroPreset(emblemY) {
  const components = [
    {
      type: "Godrays",
      props: {
        center: { x: 0.5, y: -0.08 },
        rayColor: "#e8b84a",
        backgroundColor: "transparent",
        density: 0.22,
        intensity: 0.55,
        spotty: 0.6,
        speed: pace(0.18),
        opacity: 0.55,
      },
    },
    {
      type: "SunBurst",
      id: "sun",
      props: {
        center: { x: 0.5, y: emblemY },
        color: "#ffcf70",
        background: "transparent",
        rayCount: 18,
        softness: 0.7,
        radius: 0.42,
        feather: 0.85,
        speed: pace(0.04),
        opacity: 0.32,
        blendMode: "screen",
      },
    },
    {
      type: "FloatingParticles",
      props: {
        count: coarsePointer ? 160 : 300,
        particleColor: "#ffe3a3",
        particleSize: 1.0,
        softness: 0.45,
        speed: pace(0.1),
        angle: 90,
        angleVariance: 40,
        twinkle: pace(0.7),
        randomness: pace(0.25),
        cursorStrength: pace(0.35),
        opacity: 0.6,
        blendMode: "screen",
      },
    },
  ];
  if (!coarsePointer && !reducedMotion) {
    components.push({
      type: "CursorTrail",
      props: {
        colorA: "#ffffff",
        colorB: "#2a7bff",
        radius: 0.55,
        length: 0.6,
        shrink: 1,
        softness: 0.45,
        opacity: 0.9,
        blendMode: "screen",
      },
    });
  }
  return { components };
}

/** Size the hero canvas from the top of the page to just below the header, and
 * return where the emblem's centre falls on it (0 at the top, 1 at the bottom). */
function fitHero(canvas, header) {
  const top = (element) => element.getBoundingClientRect().top + window.scrollY;
  const height = Math.ceil(top(header) + header.offsetHeight + 120);
  canvas.style.height = `${height}px`;
  const emblem = header.querySelector(".emblem");
  return emblem ? (top(emblem) + emblem.offsetHeight / 2) / height : 0.25;
}

async function start() {
  if (!navigator.gpu) return;
  const header = document.querySelector(".hero");
  if (!header) return;

  let createShader;
  try {
    const library = await import(LIBRARY);
    if (!(await library.getWebGPUSupport()).supported) return;
    createShader = library.createShader;
  } catch (error) {
    console.info("Site effects unavailable:", error);
    return;
  }

  const canvas = document.createElement("canvas");
  canvas.className = "fx fx-hero";
  canvas.setAttribute("aria-hidden", "true");
  document.body.prepend(canvas);
  const emblemY = fitHero(canvas, header);
  try {
    const shader = await createShader(canvas, heroPreset(emblemY), {
      disableTelemetry: true,
      onReady: () => canvas.classList.add("fx-ready"),
      // A terminal failure leaves the canvas empty: drop it, the CSS backdrop
      // stays. Other reasons are a lost device the library is already rebuilding.
      onError: (reason) => {
        if (TERMINAL.has(reason)) canvas.remove();
      },
    });
    // A still scene runs just long enough to draw, and again after a resize.
    let hold = 0;
    const holdStill = () => {
      if (!reducedMotion) return;
      clearTimeout(hold);
      shader.resume();
      hold = setTimeout(() => shader.pause(), STILL_AFTER_MS);
    };
    holdStill();
    window.addEventListener("resize", () => {
      shader.update("sun", { center: { x: 0.5, y: fitHero(canvas, header) } });
      holdStill();
    }, { passive: true });
  } catch (error) {
    canvas.remove();
    console.info("Site effects failed:", error);
  }
}

start();
