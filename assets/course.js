/* ==========================================================================
   Learn egui — Reusable Interactive Course Components
   Shared across ./lessons/*.html and ./reference/*.html
   Includes shared loader for Prism.js (syntax highlighting) & Mermaid.js
   ========================================================================== */

/**
 * 0. Shared Prism.js & Mermaid.js Loader
 * Dynamically loads Prism (with Rust, TOML, Bash, and keep-markup support)
 * and Mermaid.js so all lessons & references get consistent highlighting
 * and diagram rendering from a single shared component.
 */
function loadScript(src) {
  return new Promise((resolve, reject) => {
    const s = document.createElement("script");
    s.src = src;
    s.async = false;
    s.onload = resolve;
    s.onerror = reject;
    document.head.appendChild(s);
  });
}

function extractCopyableText(codeEl) {
  const clone = codeEl.cloneNode(true);
  clone.querySelectorAll("input").forEach((inp, idx) => {
    const original = codeEl.querySelectorAll("input")[idx];
    const currentVal = original && original.value ? original.value.trim() : "";
    const fallback = inp.getAttribute("placeholder") || "";
    inp.replaceWith(document.createTextNode(currentVal || fallback));
  });
  return clone.textContent || "";
}

async function copyTextToClipboard(text) {
  if (navigator.clipboard && navigator.clipboard.writeText) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch (_) {
      // Fall through to textarea fallback (e.g., on file:// protocol)
    }
  }
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.top = "-9999px";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();
  let ok = false;
  try {
    ok = document.execCommand("copy");
  } catch (_) {
    ok = false;
  }
  document.body.removeChild(textarea);
  return ok;
}

function getOrCreateCopyStatusAnnouncer() {
  let announcer = document.getElementById("copy-live-announcer");
  if (!announcer) {
    announcer = document.createElement("div");
    announcer.id = "copy-live-announcer";
    announcer.className = "visually-hidden";
    announcer.setAttribute("aria-live", "polite");
    announcer.setAttribute("aria-atomic", "true");
    document.body.appendChild(announcer);
  }
  return announcer;
}

function initCopyButtons() {
  const announcer = getOrCreateCopyStatusAnnouncer();
  const preBlocks = document.querySelectorAll("pre:has(> code)");

  preBlocks.forEach((pre, index) => {
    if (pre.closest(".code-block-wrapper")) return;

    const codeEl = pre.querySelector("code");
    if (!codeEl) return;

    const wrapper = document.createElement("div");
    wrapper.className = "code-block-wrapper";
    if (pre.parentElement && pre.parentElement.classList.contains("sim-box")) {
      wrapper.classList.add("is-compact");
    }
    if (pre.style.marginBottom) {
      wrapper.style.marginBottom = pre.style.marginBottom;
      pre.style.marginBottom = "0";
    }

    pre.parentNode.insertBefore(wrapper, pre);
    wrapper.appendChild(pre);

    const langMatch = (codeEl.className || "").match(/language-([a-z0-9_-]+)/i);
    const langName = langMatch ? langMatch[1].toUpperCase() : "TEXT";

    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "copy-code-btn";
    btn.setAttribute(
      "aria-label",
      `Copy ${langName} code block ${index + 1} to clipboard`
    );

    const renderIdleContent = () => {
      btn.innerHTML = `<svg aria-hidden="true" width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="5.5" y="5.5" width="8.5" height="8.5" rx="1.5"></rect><path d="M10.5 5.5V3.5C10.5 2.67 9.83 2 9 2H3.5C2.67 2 2 2.67 2 3.5V9C2 9.83 2.67 10.5 3.5 10.5H5.5"></path></svg><span>Copy</span>`;
    };

    const renderCopiedContent = () => {
      btn.innerHTML = `<svg aria-hidden="true" width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3.5 8.5 6.5 11.5 12.5 4.5"></polyline></svg><span>Copied!</span>`;
    };

    renderIdleContent();

    let resetTimer = null;
    btn.addEventListener("click", async () => {
      const text = extractCopyableText(codeEl);
      const ok = await copyTextToClipboard(text);
      if (!ok) return;

      if (resetTimer) clearTimeout(resetTimer);
      btn.classList.add("copied");
      renderCopiedContent();
      announcer.textContent = `Copied ${langName} code block ${index + 1} to clipboard.`;

      resetTimer = setTimeout(() => {
        btn.classList.remove("copied");
        renderIdleContent();
        resetTimer = null;
      }, 1800);
    });

    wrapper.appendChild(btn);
  });
}

async function initSyntaxAndDiagrams() {
  // Ensure scrollable code blocks are keyboard reachable and have copy buttons
  document.querySelectorAll("pre:has(> code)").forEach((pre) => {
    if (!pre.hasAttribute("tabindex")) {
      pre.setAttribute("tabindex", "0");
    }
  });
  initCopyButtons();

  try {
    // Load Prism.js core + languages + keep-markup (preserves <input> inside <pre><code>)
    window.Prism = window.Prism || {};
    window.Prism.manual = true;
    await loadScript("https://cdn.jsdelivr.net/npm/prismjs@1.29.0/prism.min.js");
    await loadScript("https://cdn.jsdelivr.net/npm/prismjs@1.29.0/components/prism-clike.min.js");
    await loadScript("https://cdn.jsdelivr.net/npm/prismjs@1.29.0/components/prism-rust.min.js");
    await loadScript("https://cdn.jsdelivr.net/npm/prismjs@1.29.0/components/prism-toml.min.js");
    await loadScript("https://cdn.jsdelivr.net/npm/prismjs@1.29.0/components/prism-bash.min.js");
    await loadScript("https://cdn.jsdelivr.net/npm/prismjs@1.29.0/plugins/keep-markup/prism-keep-markup.min.js");
    if (window.Prism && window.Prism.highlightAll) {
      window.Prism.highlightAll();
    }
  } catch (err) {
    console.warn("Prism.js failed to load from CDN:", err);
  }

  try {
    // Load Mermaid.js and render all <pre class="mermaid"> diagrams
    await loadScript("https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js");
    if (window.mermaid) {
      window.mermaid.initialize({
        startOnLoad: false,
        theme: "dark",
        securityLevel: "loose",
        fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
      });
      await window.mermaid.run({
        querySelector: "pre.mermaid",
      });
    }
  } catch (err) {
    console.warn("Mermaid.js failed to load from CDN:", err);
  }
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initSyntaxAndDiagrams);
} else {
  initSyntaxAndDiagrams();
}

/**
 * 1. Immediate-Mode PCB Layer Frame Simulator
 * Demonstrates how `eframe::App::update(&mut self, ctx, frame)` reads `self`,
 * handles widget interaction via `&mut`, and emits draw commands every frame.
 */
function initPcbLayerSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    frameCount: 1,
    showTopCopper: true,
    showBottomCopper: true,
    showSilkscreen: true,
    showRatsnest: false,
    lastEvent: "Initial frame rendered from PcbReviewerApp::default()",
  };

  function render() {
    // Update Rust struct state inspector and re-highlight with Prism if available
    const stateEl = root.querySelector("[data-role='rust-state']");
    if (stateEl) {
      stateEl.textContent = `struct PcbReviewerApp {
    show_top_copper: ${state.showTopCopper},
    show_bottom_copper: ${state.showBottomCopper},
    show_silkscreen: ${state.showSilkscreen},
    show_ratsnest: ${state.showRatsnest},
}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }

    // Update frame counter badge
    const badgeEl = root.querySelector("[data-role='frame-badge']");
    if (badgeEl) {
      badgeEl.textContent = `Frame #${state.frameCount} • update(&mut self, ctx, frame)`;
    }

    // Update checkboxes in mock egui SidePanel
    const syncCheck = (key, prop) => {
      const cb = root.querySelector(`input[data-layer='${key}']`);
      if (cb) cb.checked = state[prop];
    };
    syncCheck("top", "showTopCopper");
    syncCheck("bottom", "showBottomCopper");
    syncCheck("silk", "showSilkscreen");
    syncCheck("rats", "showRatsnest");

    // Update SVG PCB layers inside CentralPanel
    const setLayerVis = (layerId, visible) => {
      const el = root.querySelector(`#${layerId}`);
      if (el) el.style.opacity = visible ? "1" : "0.06";
    };
    setLayerVis("svg-layer-bottom", state.showBottomCopper);
    setLayerVis("svg-layer-top", state.showTopCopper);
    setLayerVis("svg-layer-silk", state.showSilkscreen);
    setLayerVis("svg-layer-rats", state.showRatsnest);

    // Update trace log
    const traceEl = root.querySelector("[data-role='frame-trace']");
    if (traceEl) {
      const activeLayers = [
        state.showBottomCopper ? "BottomCu" : null,
        state.showTopCopper ? "TopCu" : null,
        state.showSilkscreen ? "Silk" : null,
        state.showRatsnest ? "Ratsnest" : null,
      ].filter(Boolean);

      traceEl.innerHTML = `
        <div style="margin-bottom:0.45rem;"><strong>Last Event:</strong> ${state.lastEvent}</div>
        <div style="font-family:var(--font-mono); font-size:0.78rem; color:var(--text-muted);">
          1. Borrow <code>&amp;mut self</code> for Frame #${state.frameCount}<br>
          2. <code>SidePanel::left</code> reads &amp; mutates bool fields<br>
          3. <code>CentralPanel</code> paints: [${activeLayers.join(", ") || "None"}]<br>
          4. End of <code>update()</code> — borrow released!
        </div>
      `;
    }
  }

  // Bind checkbox listeners
  const bindCheck = (key, prop, rustField) => {
    const cb = root.querySelector(`input[data-layer='${key}']`);
    if (!cb) return;
    cb.addEventListener("change", () => {
      state[prop] = cb.checked;
      state.frameCount += 1;
      state.lastEvent = `ui.checkbox(&mut self.${rustField}, ...) wrote ${cb.checked}`;
      render();
    });
  };

  bindCheck("top", "showTopCopper", "show_top_copper");
  bindCheck("bottom", "showBottomCopper", "show_bottom_copper");
  bindCheck("silk", "showSilkscreen", "show_silkscreen");
  bindCheck("rats", "showRatsnest", "show_ratsnest");

  // Bind buttons
  const allOnBtn = root.querySelector("[data-action='all-on']");
  if (allOnBtn) {
    allOnBtn.addEventListener("click", () => {
      state.showTopCopper = true;
      state.showBottomCopper = true;
      state.showSilkscreen = true;
      state.showRatsnest = true;
      state.frameCount += 1;
      state.lastEvent = `ui.button("Show All").clicked() == true -> set all fields true`;
      render();
    });
  }

  const copperOnlyBtn = root.querySelector("[data-action='copper-only']");
  if (copperOnlyBtn) {
    copperOnlyBtn.addEventListener("click", () => {
      state.showTopCopper = true;
      state.showBottomCopper = false;
      state.showSilkscreen = false;
      state.showRatsnest = false;
      state.frameCount += 1;
      state.lastEvent = `ui.button("Isolate Top Copper").clicked() == true -> self.isolate_top_copper()`;
      render();
    });
  }

  render();
}

/**
 * 2. Interactive Rust + egui Code Completion Exercise
 * Validates user-typed Rust expressions with immediate, specific feedback.
 */
function initCodeExercise(config) {
  const checkBtn = document.getElementById(config.checkBtnId);
  const hintBtn = document.getElementById(config.hintBtnId);
  const feedbackEl = document.getElementById(config.feedbackId);
  if (!checkBtn || !feedbackEl) return;

  const normalize = (s) => s.replace(/\s+/g, " ").trim();

  checkBtn.addEventListener("click", () => {
    const results = config.blanks.map((b) => {
      const input = document.getElementById(b.inputId);
      const val = input ? normalize(input.value) : "";
      const accepted = b.accepted.map(normalize);
      const isOk = accepted.includes(val);
      return { ...b, val, isOk };
    });

    const firstWrong = results.find((r) => !r.isOk);
    if (!firstWrong) {
      feedbackEl.className = "feedback-box ok";
      feedbackEl.innerHTML = config.successMessage;
    } else {
      feedbackEl.className = "feedback-box err";
      feedbackEl.innerHTML = firstWrong.getErrorHint(firstWrong.val);
    }
  });

  if (hintBtn) {
    hintBtn.addEventListener("click", () => {
      feedbackEl.className = "feedback-box ok";
      feedbackEl.innerHTML = config.hintMessage;
    });
  }
}

/**
 * 3. Desirable-Difficulty Retrieval Quiz Component
 * Renders multiple-choice questions where every option has matched word/char length
 * and varied correct-answer positions.
 */
function initQuiz(containerId, questions) {
  const container = document.getElementById(containerId);
  if (!container) return;

  container.innerHTML = "";
  questions.forEach((q, qIdx) => {
    const card = document.createElement("div");
    card.className = "quiz-question";

    const prompt = document.createElement("div");
    prompt.className = "quiz-prompt";
    prompt.textContent = `${qIdx + 1}. ${q.question}`;
    card.appendChild(prompt);

    const optsGrid = document.createElement("div");
    optsGrid.className = "quiz-options";

    const feedback = document.createElement("div");
    feedback.className = "feedback-box";

    const buttons = [];
    q.options.forEach((optText, optIdx) => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "quiz-option-btn";
      const letter = String.fromCharCode(65 + optIdx);
      btn.textContent = `${letter}. ${optText}`;

      btn.addEventListener("click", () => {
        buttons.forEach((b) => {
          b.classList.remove("selected-correct", "selected-wrong");
        });
        if (optIdx === q.correctIndex) {
          btn.classList.add("selected-correct");
          feedback.className = "feedback-box ok";
          feedback.innerHTML = `<strong>Correct (${letter}):</strong> ${q.explanation}`;
        } else {
          btn.classList.add("selected-wrong");
          feedback.className = "feedback-box err";
          feedback.innerHTML = `<strong>Not quite (${letter}):</strong> ${q.wrongHint}`;
        }
      });

      buttons.push(btn);
      optsGrid.appendChild(btn);
    });

    card.appendChild(optsGrid);
    card.appendChild(feedback);
    container.appendChild(card);
  });
}

/**
 * 4. Lesson 0002 Simulator: 2D Painter Primitives & Rect Geometry
 */
function initPainterCanvasSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    margin: 24,
    padW: 22,
    padH: 14,
    viaRadius: 7,
  };

  function render() {
    const canvasW = 320;
    const canvasH = 210;
    const minX = state.margin;
    const minY = state.margin;
    const maxX = canvasW - state.margin;
    const maxY = canvasH - state.margin;

    const padCenter = { x: minX + 64, y: minY + 56 };
    const padMinX = padCenter.x - state.padW / 2;
    const padMinY = padCenter.y - state.padH / 2;
    const padMaxX = padCenter.x + state.padW / 2;
    const padMaxY = padCenter.y + state.padH / 2;

    const viaPos = { x: minX + 190, y: minY + 86 };

    const boardRectEl = root.querySelector("[data-svg='board-rect']");
    if (boardRectEl) {
      boardRectEl.setAttribute("x", minX);
      boardRectEl.setAttribute("y", minY);
      boardRectEl.setAttribute("width", Math.max(20, maxX - minX));
      boardRectEl.setAttribute("height", Math.max(20, maxY - minY));
    }

    const padRectEl = root.querySelector("[data-svg='pad-rect']");
    if (padRectEl) {
      padRectEl.setAttribute("x", padMinX);
      padRectEl.setAttribute("y", padMinY);
      padRectEl.setAttribute("width", state.padW);
      padRectEl.setAttribute("height", state.padH);
    }

    const trace1El = root.querySelector("[data-svg='trace-1']");
    if (trace1El) {
      trace1El.setAttribute("x1", padCenter.x);
      trace1El.setAttribute("y1", padCenter.y);
      trace1El.setAttribute("x2", padCenter.x + 90);
      trace1El.setAttribute("y2", padCenter.y);
    }

    const trace2El = root.querySelector("[data-svg='trace-2']");
    if (trace2El) {
      trace2El.setAttribute("x1", padCenter.x + 90);
      trace2El.setAttribute("y1", padCenter.y);
      trace2El.setAttribute("x2", viaPos.x);
      trace2El.setAttribute("y2", viaPos.y);
    }

    const viaEl = root.querySelector("[data-svg='via-circle']");
    if (viaEl) {
      viaEl.setAttribute("cx", viaPos.x);
      viaEl.setAttribute("cy", viaPos.y);
      viaEl.setAttribute("r", state.viaRadius);
    }

    const stateEl = root.querySelector("[data-role='painter-state']");
    if (stateEl) {
      stateEl.textContent = `// Screen-space Rects (Copy values)
let board = centered_board_rect(
    canvas, ${state.margin.toFixed(1)}
);
// min: pos2(${minX.toFixed(1)}, ${minY.toFixed(1)})
// max: pos2(${maxX.toFixed(1)}, ${maxY.toFixed(1)})

let pad = smd_pad_rect(
    pos2(${padCenter.x.toFixed(1)}, ${padCenter.y.toFixed(1)}),
    vec2(${state.padW.toFixed(1)}, ${state.padH.toFixed(1)}),
);
// min: pos2(${padMinX.toFixed(1)}, ${padMinY.toFixed(1)})
// max: pos2(${padMaxX.toFixed(1)}, ${padMaxY.toFixed(1)})`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }

    const traceEl = root.querySelector("[data-role='painter-trace']");
    if (traceEl) {
      traceEl.innerHTML = `
        <div style="font-family:var(--font-mono); font-size:0.78rem; color:var(--text-muted); line-height:1.6;">
          1. <code>painter.rect(board, 6.0, ...)</code><br>
          2. <code>painter.line_segment([p0, p1], stroke)</code><br>
          3. <code>painter.rect_filled(pad, 2.0, f_cu)</code><br>
          4. <code>painter.circle(pos2(${viaPos.x}, ${viaPos.y}), ${state.viaRadius}.0, ...)</code>
        </div>
      `;
    }
  }

  ["margin", "padW", "padH", "viaRadius"].forEach((key) => {
    const input = root.querySelector(`input[data-param='${key}']`);
    if (input) {
      input.addEventListener("input", () => {
        state[key] = parseFloat(input.value);
        render();
      });
    }
  });

  render();
}

/**
 * 5. Lesson 0003 Simulator: World-to-Screen TSTransform (Pan & Cursor Zoom)
 */
function initViewportTransformSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    scaling: 3.0,
    tx: 24.0,
    ty: 20.0,
    cursorScreen: { x: 160.0, y: 105.0 },
    isDragging: false,
    lastMouse: { x: 0, y: 0 },
  };

  const worldToScreen = (wx, wy) => ({
    x: wx * state.scaling + state.tx,
    y: wy * state.scaling + state.ty,
  });

  const screenToWorld = (sx, sy) => ({
    x: (sx - state.tx) / state.scaling,
    y: (sy - state.ty) / state.scaling,
  });

  function zoomAround(sx, sy, factor) {
    const beforeWorld = screenToWorld(sx, sy);
    const nextScale = Math.min(12.0, Math.max(1.5, state.scaling * factor));
    state.scaling = nextScale;
    state.tx = sx - beforeWorld.x * nextScale;
    state.ty = sy - beforeWorld.y * nextScale;
    render();
  }

  function render() {
    const g = root.querySelector("[data-svg='world-group']");
    if (g) {
      g.setAttribute(
        "transform",
        `translate(${state.tx.toFixed(2)}, ${state.ty.toFixed(2)}) scale(${state.scaling.toFixed(3)})`
      );
    }

    const cursorEl = root.querySelector("[data-svg='cursor-crosshair']");
    if (cursorEl) {
      cursorEl.setAttribute(
        "transform",
        `translate(${state.cursorScreen.x.toFixed(1)}, ${state.cursorScreen.y.toFixed(1)})`
      );
    }

    const wCursor = screenToWorld(state.cursorScreen.x, state.cursorScreen.y);
    const padScreen = worldToScreen(20.0, 22.0);

    const stateEl = root.querySelector("[data-role='transform-state']");
    if (stateEl) {
      stateEl.textContent = `BoardViewport {
    transform: TSTransform {
        scaling: ${state.scaling.toFixed(2)}, // px/mm
        translation: vec2(${state.tx.toFixed(1)}, ${state.ty.toFixed(1)}),
    },
}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }

    const badgeEl = root.querySelector("[data-role='zoom-badge']");
    if (badgeEl) {
      badgeEl.textContent = `Zoom: ${state.scaling.toFixed(2)} px/mm`;
    }

    const traceEl = root.querySelector("[data-role='transform-math']");
    if (traceEl) {
      traceEl.innerHTML = `
        <div style="font-family:var(--font-mono); font-size:0.78rem; line-height:1.6;">
          <strong>Cursor Screen (px):</strong> (${state.cursorScreen.x.toFixed(1)}, ${state.cursorScreen.y.toFixed(1)})<br>
          <strong>screen_to_world (mm):</strong> (${wCursor.x.toFixed(2)} mm, ${wCursor.y.toFixed(2)} mm)<br>
          <strong>Pad (20, 22) mm → px:</strong> (${padScreen.x.toFixed(1)}, ${padScreen.y.toFixed(1)})
        </div>
      `;
    }
  }

  const svg = root.querySelector("svg[data-role='viewport-svg']");
  if (svg) {
    const getSvgPt = (evt) => {
      const rect = svg.getBoundingClientRect();
      return {
        x: ((evt.clientX - rect.left) / rect.width) * 320,
        y: ((evt.clientY - rect.top) / rect.height) * 210,
      };
    };

    svg.addEventListener("mousedown", (e) => {
      state.isDragging = true;
      state.lastMouse = getSvgPt(e);
    });

    window.addEventListener("mouseup", () => {
      state.isDragging = false;
    });

    svg.addEventListener("mousemove", (e) => {
      const pt = getSvgPt(e);
      state.cursorScreen = pt;
      if (state.isDragging) {
        state.tx += pt.x - state.lastMouse.x;
        state.ty += pt.y - state.lastMouse.y;
        state.lastMouse = pt;
      }
      render();
    });
  }

  const btnIn = root.querySelector("[data-action='zoom-in']");
  if (btnIn) {
    btnIn.addEventListener("click", () =>
      zoomAround(state.cursorScreen.x, state.cursorScreen.y, 1.25)
    );
  }
  const btnOut = root.querySelector("[data-action='zoom-out']");
  if (btnOut) {
    btnOut.addEventListener("click", () =>
      zoomAround(state.cursorScreen.x, state.cursorScreen.y, 0.8)
    );
  }
  const btnReset = root.querySelector("[data-action='zoom-reset']");
  if (btnReset) {
    btnReset.addEventListener("click", () => {
      state.scaling = 3.0;
      state.tx = 24.0;
      state.ty = 20.0;
      render();
    });
  }

  render();
}

/**
 * 6. Lesson 0004 Simulator: PcbLayer Z-Rank Sorting & Visibility Filtering
 */
function initLayerZOrderSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    showBottom: true,
    showTop: true,
    showSilk: true,
    showRats: true,
    sortByZRank: true,
  };

  // Raw input order deliberately has Ratsnest & BottomCopper in reverse order
  const rawSegments = [
    { id: "rats", layer: "Ratsnest", zRank: 3, color: "#6ff7ff", label: "Ratsnest (z=3)" },
    { id: "silk", layer: "Silkscreen", zRank: 2, color: "#f0e68c", label: "Silkscreen (z=2)" },
    { id: "top", layer: "TopCopper", zRank: 1, color: "#e05243", label: "TopCopper (z=1)" },
    { id: "bottom", layer: "BottomCopper", zRank: 0, color: "#3b8eea", label: "BottomCopper (z=0)" },
  ];

  function isVisible(layer) {
    if (layer === "BottomCopper") return state.showBottom;
    if (layer === "TopCopper") return state.showTop;
    if (layer === "Silkscreen") return state.showSilk;
    if (layer === "Ratsnest") return state.showRats;
    return false;
  }

  function render() {
    let filtered = rawSegments.filter((s) => isVisible(s.layer));
    if (state.sortByZRank) {
      filtered = [...filtered].sort((a, b) => a.zRank - b.zRank);
    }

    const svgGroup = root.querySelector("[data-svg='z-layers']");
    if (svgGroup) {
      svgGroup.innerHTML = "";
      filtered.forEach((seg) => {
        if (seg.layer === "BottomCopper") {
          svgGroup.innerHTML += `<path d="M 60 110 L 250 110" stroke="#3b8eea" stroke-width="18" stroke-linecap="round"/>`;
        } else if (seg.layer === "TopCopper") {
          svgGroup.innerHTML += `<path d="M 155 40 L 155 175" stroke="#e05243" stroke-width="18" stroke-linecap="round"/>`;
        } else if (seg.layer === "Silkscreen") {
          svgGroup.innerHTML += `<rect x="110" y="65" width="90" height="85" fill="none" stroke="#f0e68c" stroke-width="4" stroke-dasharray="6 4"/>`;
        } else if (seg.layer === "Ratsnest") {
          svgGroup.innerHTML += `<line x1="75" y1="55" x2="235" y2="160" stroke="#6ff7ff" stroke-width="3" stroke-dasharray="5 5"/>`;
        }
      });
    }

    const stateEl = root.querySelector("[data-role='zorder-state']");
    if (stateEl) {
      const orderStr = filtered.map((s) => `    PcbLayer::${s.layer}, // z_rank = ${s.zRank}`).join("\n");
      stateEl.textContent = `// Draw order (${state.sortByZRank ? "sorted by z_rank" : "UNSORTED raw Vec"}):
vec![
${orderStr || "    // (all layers hidden)"}
]`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  const bindToggle = (selector, prop) => {
    const el = root.querySelector(selector);
    if (el) {
      el.addEventListener("change", () => {
        state[prop] = el.checked;
        render();
      });
    }
  };

  bindToggle("input[data-z='bottom']", "showBottom");
  bindToggle("input[data-z='top']", "showTop");
  bindToggle("input[data-z='silk']", "showSilk");
  bindToggle("input[data-z='rats']", "showRats");
  bindToggle("input[data-z='sort']", "sortByZRank");

  render();
}

/**
 * 7. Lesson 0005 Simulator: Hit-Testing & NetId Cross-Probing
 */
function initHitTestCrossProbeSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    hoveredNet: null,
    selectedNet: null,
  };

  const activeNet = () => (state.selectedNet !== null ? state.selectedNet : state.hoveredNet);

  function render() {
    const active = activeNet();

    [1, 2].forEach((netId) => {
      const isHi = active === netId;
      root.querySelectorAll(`[data-net='${netId}']`).forEach((el) => {
        if (el.tagName.toLowerCase() === "rect") {
          el.setAttribute("fill", isHi ? "#ffe664" : el.getAttribute("data-base-color"));
        } else {
          el.setAttribute("stroke", isHi ? "#ffe664" : el.getAttribute("data-base-color"));
          el.setAttribute("stroke-width", isHi ? "7" : "5");
        }
      });
    });

    const fmtOpt = (v) => (v === null ? "None" : `Some(NetId(${v}))`);
    const stateEl = root.querySelector("[data-role='hittest-state']");
    if (stateEl) {
      stateEl.textContent = `hovered_net: ${fmtOpt(state.hoveredNet)},
selected_net: ${fmtOpt(state.selectedNet)},
// self.selected_net.or(self.hovered_net)
active_net(): ${fmtOpt(active)},`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }

    const inspectorEl = root.querySelector("[data-role='net-inspector']");
    if (inspectorEl) {
      if (active === 1) {
        inspectorEl.innerHTML = `<strong>Active Net:</strong> <code>NET_1 (VCC_3V3)</code><br>Connected: <code>U1.1</code> → Via → <code>J1.1</code>`;
      } else if (active === 2) {
        inspectorEl.innerHTML = `<strong>Active Net:</strong> <code>NET_2 (SPI_CLK)</code><br>Connected: <code>U1.2</code> → <code>J1.2</code>`;
      } else {
        inspectorEl.innerHTML = `Hover or click any pad on the canvas to cross-probe its net.`;
      }
    }
  }

  root.querySelectorAll("[data-hit-net]").forEach((el) => {
    const netId = parseInt(el.getAttribute("data-hit-net"), 10);
    el.addEventListener("mouseenter", () => {
      state.hoveredNet = netId;
      render();
    });
    el.addEventListener("mouseleave", () => {
      state.hoveredNet = null;
      render();
    });
    el.addEventListener("click", (e) => {
      e.stopPropagation();
      state.selectedNet = netId;
      render();
    });
  });

  const clearBtn = root.querySelector("[data-action='clear-net']");
  if (clearBtn) {
    clearBtn.addEventListener("click", () => {
      state.selectedNet = null;
      render();
    });
  }

  render();
}

/**
 * 8. Lesson 0006 Simulator: Painter::text, Align2 & egui_kittest Accessibility Tree
 */
function initSchematicTextSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    align: "CENTER_CENTER",
    fontSize: 13,
    showSilkscreen: true,
    selectedNet: 1,
  };

  const alignMap = {
    CENTER_CENTER: { anchor: "middle", dy: "0.35em", dx: 0 },
    LEFT_TOP: { anchor: "start", dy: "0.9em", dx: 0 },
    RIGHT_BOTTOM: { anchor: "end", dy: "-0.15em", dx: 0 },
  };

  function render() {
    const a = alignMap[state.align] || alignMap.CENTER_CENTER;

    root.querySelectorAll("[data-svg='refdes-label']").forEach((el) => {
      el.setAttribute("text-anchor", a.anchor);
      el.setAttribute("dy", a.dy);
      el.setAttribute("font-size", state.fontSize);
      el.style.opacity = state.showSilkscreen ? "1" : "0.08";
    });

    const silkGroup = root.querySelector("[data-svg='silk-outlines']");
    if (silkGroup) {
      silkGroup.style.opacity = state.showSilkscreen ? "1" : "0.12";
    }

    const codeEl = root.querySelector("[data-role='text-painter-code']");
    if (codeEl) {
      const netLine =
        state.selectedNet === 1
          ? `Some(NetId(1)) => "Active Net: NET_1 (VCC_3V3)"`
          : state.selectedNet === 2
          ? `Some(NetId(2)) => "Active Net: NET_2 (SPI_CLK)"`
          : `None => "Hover or click a pad..."`;
      codeEl.textContent = `// 1. Canvas text via Painter::text (&str + FontId + Align2)
painter.text(
    anchor_px,
    Align2::${state.align},
    &comp.ref_des, // &String -> &str coercion
    FontId::monospace(${state.fontSize.toFixed(1)}),
    Color32::from_rgb(240, 230, 140),
);

// 2. SidePanel format!() -> owned String
// ${netLine}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(codeEl);
      }
    }

    const kittestEl = root.querySelector("[data-role='kittest-tree']");
    if (kittestEl) {
      const silkNodes = state.showSilkscreen
        ? `  ├─ Label("U1: LDO_3V3 (SOT-23)")  [FOUND]\n  ├─ Label("R12: 10k (0603)")       [FOUND]\n  ├─ Label("J1: CONN_2P (Header)")  [FOUND]\n`
        : `  ├─ (Silkscreen hidden: 0 RefDes labels in tree)\n`;
      const netNode =
        state.selectedNet === 1
          ? `  ├─ Label("Active Net: NET_1 (VCC_3V3)")\n  └─ Button("Clear Selection")`
          : state.selectedNet === 2
          ? `  ├─ Label("Active Net: NET_2 (SPI_CLK)")\n  └─ Button("Clear Selection")`
          : `  └─ Label("Hover or click a pad to cross-probe its net.")`;

      kittestEl.textContent = `Harness::new_eframe(|_cc| PcbReviewerApp::default())
SidePanel("layer_panel")
  ├─ CheckBox("Top Copper (F.Cu)")
  ├─ CheckBox("Silkscreen (F.SilkS)") = ${state.showSilkscreen}
${silkNodes}${netNode}`;
    }
  }

  const alignSel = root.querySelector("[data-param='align']");
  if (alignSel) {
    alignSel.addEventListener("change", () => {
      state.align = alignSel.value;
      render();
    });
  }

  const sizeInput = root.querySelector("[data-param='fontSize']");
  if (sizeInput) {
    sizeInput.addEventListener("input", () => {
      state.fontSize = parseFloat(sizeInput.value);
      render();
    });
  }

  const silkCheck = root.querySelector("[data-param='silk']");
  if (silkCheck) {
    silkCheck.addEventListener("change", () => {
      state.showSilkscreen = silkCheck.checked;
      render();
    });
  }

  root.querySelectorAll("[data-select-net]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const v = btn.getAttribute("data-select-net");
      state.selectedNet = v === "none" ? null : parseInt(v, 10);
      render();
    });
  });

  render();
}

/**
 * 9. Lesson 0007 Simulator: Dual-Pane Schematic + PCB egui::Scene Cross-Probing
 */
function initDualSceneCrossProbeSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const components = {
    U1: {
      refDes: "U1",
      value: "LDO_3V3",
      netId: 1,
      pcbCenter: { x: 20.0, y: 27.0 },
      svgTarget: { x: 78, y: 80 },
    },
    R12: {
      refDes: "R12",
      value: "10k",
      netId: 2,
      pcbCenter: { x: 52.0, y: 35.0 },
      svgTarget: { x: 162, y: 112 },
    },
    J1: {
      refDes: "J1",
      value: "CONN_2P",
      netId: 1,
      pcbCenter: { x: 68.0, y: 35.0 },
      svgTarget: { x: 242, y: 102 },
    },
  };

  const state = {
    selectedRef: "U1",
  };

  function render() {
    const comp = components[state.selectedRef] || components.U1;

    root.querySelectorAll("[data-sch-comp]").forEach((el) => {
      const ref = el.getAttribute("data-sch-comp");
      const active = ref === state.selectedRef;
      el.setAttribute("stroke", active ? "#ffe664" : "#6cb6ff");
      el.setAttribute("stroke-width", active ? "3" : "1.8");
    });

    root.querySelectorAll("[data-pcb-comp]").forEach((el) => {
      const ref = el.getAttribute("data-pcb-comp");
      const active = ref === state.selectedRef;
      el.setAttribute("stroke", active ? "#ffe664" : "#f0e68c");
      el.setAttribute("stroke-width", active ? "3" : "1.6");
    });

    const camBox = root.querySelector("[data-svg='pcb-camera-box']");
    if (camBox) {
      camBox.setAttribute("x", comp.svgTarget.x - 36);
      camBox.setAttribute("y", comp.svgTarget.y - 28);
    }

    const stateEl = root.querySelector("[data-role='scene-state']");
    if (stateEl) {
      const minX = (comp.pcbCenter.x - 20.0).toFixed(1);
      const minY = (comp.pcbCenter.y - 15.0).toFixed(1);
      const maxX = (comp.pcbCenter.x + 20.0).toFixed(1);
      const maxY = (comp.pcbCenter.y + 15.0).toFixed(1);
      stateEl.textContent = `// Split borrow: &self.board (shared) + &mut self.view (mutable)
self.view.focus_component(&self.board, "${comp.refDes}");

ViewState {
    selected_component: Some("${comp.refDes}"),
    selected_net: Some(NetId(${comp.netId})),
    pcb_scene_rect: Rect {
        min: pos2(${minX}, ${minY}),
        max: pos2(${maxX}, ${maxY}),
    },
}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  root.querySelectorAll("[data-probe-ref]").forEach((el) => {
    el.addEventListener("click", () => {
      state.selectedRef = el.getAttribute("data-probe-ref");
      render();
    });
  });

  render();
}

/**
 * 10. Lesson 0008 Simulator: Searchable BOM & Virtualized ScrollArea::show_rows
 */
function initBomVirtualScrollSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const allRows = [
    { ref: "U1", val: "LDO_3V3", pkg: "SOT-23", net: "VCC_3V3", netId: 1 },
    { ref: "U2", val: "MCU_RP2040", pkg: "QFN-56", net: "SPI_CLK", netId: 2 },
    { ref: "R12", val: "10k_1%", pkg: "0603", net: "SPI_CLK", netId: 2 },
    { ref: "R15", val: "4.7k_I2C", pkg: "0603", net: "VCC_3V3", netId: 1 },
    { ref: "C1", val: "100nF_X7R", pkg: "0402", net: "GND", netId: 3 },
    { ref: "C2", val: "10uF_MLCC", pkg: "0805", net: "VCC_3V3", netId: 1 },
    { ref: "J1", val: "CONN_2P", pkg: "Header_1x02", net: "VCC_3V3", netId: 1 },
    { ref: "D1", val: "LED_GREEN", pkg: "0603", net: "STATUS_LED", netId: 4 },
  ];

  const state = {
    query: "",
    selectedRef: "U1",
  };

  function render() {
    const q = state.query.trim().toLowerCase();
    const filtered = allRows.filter(
      (r) =>
        q === "" ||
        r.ref.toLowerCase().includes(q) ||
        r.val.toLowerCase().includes(q) ||
        r.net.toLowerCase().includes(q)
    );

    const tbody = root.querySelector("[data-role='bom-tbody']");
    if (tbody) {
      tbody.innerHTML = "";
      filtered.forEach((row, idx) => {
        const tr = document.createElement("tr");
        const isSel = row.ref === state.selectedRef;
        tr.style.background = isSel ? "#193529" : "transparent";
        tr.style.cursor = "pointer";
        tr.innerHTML = `
          <td style="padding:0.35rem 0.5rem; font-family:var(--font-mono);">${idx}</td>
          <td style="padding:0.35rem 0.5rem; font-family:var(--font-mono); font-weight:700; color:var(--accent-copper);">${row.ref}</td>
          <td style="padding:0.35rem 0.5rem;">${row.val}</td>
          <td style="padding:0.35rem 0.5rem; font-family:var(--font-mono); color:var(--accent-silk);">${row.net}</td>
        `;
        tr.addEventListener("click", () => {
          state.selectedRef = row.ref;
          render();
        });
        tbody.appendChild(tr);
      });
    }

    const stateEl = root.querySelector("[data-role='bom-state']");
    if (stateEl) {
      stateEl.textContent = `// Query filter: .filter() + case-insensitive match
bom_search_query: "${state.query}",
matching_rows: ${filtered.length} / ${allRows.length},

// ScrollArea::vertical().show_rows(ui, 22.0, ${filtered.length}, |ui, row_range| {
//     for (idx, comp) in filtered[row_range].iter().enumerate() { ... }
// })
selected_component: Some("${state.selectedRef}"),`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  const input = root.querySelector("[data-role='bom-query-input']");
  if (input) {
    input.addEventListener("input", () => {
      state.query = input.value;
      render();
    });
  }

  const clearBtn = root.querySelector("[data-action='clear-bom-query']");
  if (clearBtn && input) {
    clearBtn.addEventListener("click", () => {
      input.value = "";
      state.query = "";
      render();
    });
  }

  render();
}

/**
 * 11. Lesson 0009 Simulator: Keyboard Shortcuts (F / M / Esc) & Point-to-Point CAD Ruler
 */
function initHotkeyRulerSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    tool: "Measure",
    startMm: { x: 20.0, y: 22.0 },
    endMm: { x: 68.0, y: 58.0 },
    zoom: 6.0,
    lastKey: "Key::M (Measure Tool)",
  };

  const mmToSvg = (pt) => ({
    x: 20 + pt.x * 2.8,
    y: 16 + pt.y * 2.5,
  });

  function render() {
    const dx = state.endMm.x - state.startMm.x;
    const dy = state.endMm.y - state.startMm.y;
    const dist = Math.hypot(dx, dy);

    const p0 = mmToSvg(state.startMm);
    const p1 = mmToSvg(state.endMm);

    const rulerGroup = root.querySelector("[data-svg='ruler-overlay']");
    if (rulerGroup) {
      rulerGroup.style.display = state.tool === "Measure" ? "block" : "none";
    }

    const lineHyp = root.querySelector("[data-svg='ruler-hyp']");
    if (lineHyp) {
      lineHyp.setAttribute("x1", p0.x);
      lineHyp.setAttribute("y1", p0.y);
      lineHyp.setAttribute("x2", p1.x);
      lineHyp.setAttribute("y2", p1.y);
    }

    const lineDx = root.querySelector("[data-svg='ruler-dx']");
    if (lineDx) {
      lineDx.setAttribute("x1", p0.x);
      lineDx.setAttribute("y1", p0.y);
      lineDx.setAttribute("x2", p1.x);
      lineDx.setAttribute("y2", p0.y);
    }

    const lineDy = root.querySelector("[data-svg='ruler-dy']");
    if (lineDy) {
      lineDy.setAttribute("x1", p1.x);
      lineDy.setAttribute("y1", p0.y);
      lineDy.setAttribute("x2", p1.x);
      lineDy.setAttribute("y2", p1.y);
    }

    const endDot = root.querySelector("[data-svg='ruler-end-dot']");
    if (endDot) {
      endDot.setAttribute("cx", p1.x);
      endDot.setAttribute("cy", p1.y);
    }

    const labelEl = root.querySelector("[data-svg='ruler-label']");
    if (labelEl) {
      labelEl.setAttribute("x", (p0.x + p1.x) / 2);
      labelEl.setAttribute("y", (p0.y + p1.y) / 2 - 8);
      labelEl.textContent = `Δx=${dx.toFixed(1)} Δy=${dy.toFixed(1)} d=${dist.toFixed(1)}mm`;
    }

    const stateEl = root.querySelector("[data-role='ruler-state']");
    if (stateEl) {
      const toolStr =
        state.tool === "Measure"
          ? `ActiveTool::Measure {\n    start_mm: Some(pos2(${state.startMm.x.toFixed(1)}, ${state.startMm.y.toFixed(1)})),\n    end_mm: Some(pos2(${state.endMm.x.toFixed(1)}, ${state.endMm.y.toFixed(1)})),\n}`
          : `ActiveTool::Select`;
      stateEl.textContent = `// Last hotkey: ${state.lastKey}
active_tool: ${toolStr},
// RulerMeasurement::between(start, end):
// dx_mm: ${dx.toFixed(1)}, dy_mm: ${dy.toFixed(1)}, distance_mm: ${dist.toFixed(1)}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  root.querySelectorAll("[data-key]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const k = btn.getAttribute("data-key");
      if (k === "M") {
        state.tool = "Measure";
        state.lastKey = "Key::M -> activate Measure tool";
      } else if (k === "Escape") {
        state.tool = "Select";
        state.lastKey = "Key::Escape -> reset to ActiveTool::Select";
      } else if (k === "F") {
        state.zoom = 6.0;
        state.lastKey = "Key::F -> fit_board_in_viewport()";
      }
      render();
    });
  });

  root.querySelectorAll("[data-ruler-preset]").forEach((btn) => {
    btn.addEventListener("click", () => {
      state.tool = "Measure";
      const [x, y] = btn
        .getAttribute("data-ruler-preset")
        .split(",")
        .map(parseFloat);
      state.endMm = { x, y };
      state.lastKey = `Measure end_mm updated to (${x.toFixed(1)}, ${y.toFixed(1)}) mm`;
      render();
    });
  });

  render();
}
