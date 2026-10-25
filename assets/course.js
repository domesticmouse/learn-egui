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

/**
 * 12. Lesson 0010 Simulator: Review Pins, HashMap::entry & Floating Callout Window
 */
function initReviewPinsSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    selectedId: 1,
    nextPinId: 3,
    lastAction: "Selected PinId(1) -> opened egui::Window callout",
    pins: [
      { id: 1, x: 20.0, y: 27.0, author: "alice", comment: "Check U1 thermal relief" },
      { id: 2, x: 68.0, y: 35.0, author: "bob", comment: "Verify J1 polarity mark" },
    ],
  };

  function render() {
    const listEl = root.querySelector("[data-role='pin-list']");
    if (listEl) {
      listEl.innerHTML = "";
      state.pins.forEach((p) => {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "egui-btn";
        btn.style.marginBottom = "0.35rem";
        btn.style.borderColor = p.id === state.selectedId ? "#ffe664" : "#3c3c3c";
        btn.textContent = `Pin #${p.id}: ${p.comment} (@${p.author})`;
        btn.addEventListener("click", () => {
          state.selectedId = p.id;
          state.lastAction = `select_review_pin(PinId(${p.id})) -> true`;
          render();
        });
        listEl.appendChild(btn);
      });
    }

    const svgGroup = root.querySelector("[data-svg='pin-markers']");
    if (svgGroup) {
      svgGroup.innerHTML = "";
      state.pins.forEach((p) => {
        const sx = 24 + p.x * 2.8;
        const sy = 18 + p.y * 2.4;
        const isSel = p.id === state.selectedId;
        const color = isSel ? "#ffe664" : "#ff8b6b";
        svgGroup.innerHTML += `
          <g data-pin-id="${p.id}" style="cursor:pointer;">
            <circle cx="${sx}" cy="${sy}" r="${isSel ? 7 : 5.5}" fill="${color}" stroke="#0c0e11" stroke-width="1.5"/>
            <text x="${sx + 9}" y="${sy + 3}" fill="${color}" font-family="monospace" font-size="10" font-weight="bold">#${p.id}</text>
          </g>`;
      });
    }

    const active = state.pins.find((p) => p.id === state.selectedId) || null;
    const calloutBox = root.querySelector("[data-svg='callout-box']");
    if (calloutBox) {
      if (active) {
        calloutBox.style.display = "block";
        const titleEl = root.querySelector("[data-svg='callout-title']");
        const coordEl = root.querySelector("[data-svg='callout-coord']");
        if (titleEl) titleEl.textContent = `Pin #${active.id}: ${active.comment}`;
        if (coordEl) coordEl.textContent = `Anchor: (${active.x.toFixed(1)}, ${active.y.toFixed(1)}) mm (@${active.author})`;
      } else {
        calloutBox.style.display = "none";
      }
    }

    const stateEl = root.querySelector("[data-role='pins-state']");
    if (stateEl) {
      const selStr = active ? `Some(PinId(${active.id}))` : "None";
      stateEl.textContent = `// ${state.lastAction}
review_pins.len(): ${state.pins.length},
next_pin_id: ${state.nextPinId},
selected_pin: ${selStr},
// Window::new("Review Callout")
//   .id(Id::new(("review_pin_callout", ${active ? active.id : 0})))`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  const addBtn = root.querySelector("[data-action='add-pin-r12']");
  if (addBtn) {
    addBtn.addEventListener("click", () => {
      if (state.pins.some((p) => p.id === 3)) {
        state.selectedId = 3;
        state.lastAction = "Entry::Occupied -> PinId(3) already exists (no overwrite!)";
      } else {
        state.pins.push({ id: 3, x: 52.0, y: 35.0, author: "carol", comment: "Check R12 value" });
        state.selectedId = 3;
        state.nextPinId = 4;
        state.lastAction = "Entry::Vacant -> inserted PinId(3) at (52.0, 35.0) mm";
      }
      render();
    });
  }

  const closeBtn = root.querySelector("[data-action='close-callout']");
  if (closeBtn) {
    closeBtn.addEventListener("click", () => {
      state.selectedId = null;
      state.lastAction = "Close Callout -> selected_pin = None";
      render();
    });
  }

  render();
}

/**
 * 13. Lesson 0011 Simulator: Review Status Triage State Machine & Multi-Criteria Filtering
 */
function initReviewTriageSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    minSeverity: "Info",
    onlyOpen: false,
    pin1Status: "Open",
    lastTransition: "Ready: Pin #1 is Open (Critical)",
  };

  const ranks = { Info: 0, Warning: 1, Critical: 2 };
  const canTransition = (cur, next) =>
    (cur === "Open" && next === "Resolved") ||
    (cur === "Resolved" && next === "Verified") ||
    (cur === "Resolved" && next === "Open") ||
    (cur === "Verified" && next === "Open");

  function render() {
    const pins = [
      { id: 1, sev: "Critical", status: state.pin1Status, text: "Check U1 thermal relief (@alice)" },
      { id: 2, sev: "Warning", status: "Resolved", text: "Verify J1 polarity mark (@bob)" },
      { id: 3, sev: "Info", status: "Open", text: "Silkscreen font size note (@carol)" },
    ];

    const filtered = pins.filter(
      (p) => ranks[p.sev] >= ranks[state.minSeverity] && (!state.onlyOpen || p.status === "Open")
    );

    const listEl = root.querySelector("[data-role='triage-list']");
    if (listEl) {
      listEl.innerHTML = filtered
        .map(
          (p) =>
            `<div style="padding:0.4rem 0.55rem; margin-bottom:0.35rem; background:#141a21; border:1px solid #2d3846; border-radius:4px; font-family:var(--font-mono); font-size:0.78rem;">
              <strong>Pin #${p.id}</strong> [${p.sev} | <span style="color:#3dd68c;">${p.status}</span>]: ${p.text}
            </div>`
        )
        .join("");
    }

    const stateEl = root.querySelector("[data-role='triage-state']");
    if (stateEl) {
      stateEl.textContent = `// ${state.lastTransition}
pin_1.status: ReviewStatus::${state.pin1Status},
min_pin_severity: PinSeverity::${state.minSeverity},
filter_only_open_pins: ${state.onlyOpen},
visible_pins: ${filtered.length} / 3,`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  root.querySelectorAll("[data-transition]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const target = btn.getAttribute("data-transition");
      if (canTransition(state.pin1Status, target)) {
        const prev = state.pin1Status;
        state.pin1Status = target;
        state.lastTransition = `can_transition_to(${prev} -> ${target}) == true`;
      } else {
        state.lastTransition = `REJECTED: ${state.pin1Status} -> ${target} (must Resolve before Verify!)`;
      }
      render();
    });
  });

  const sevSelect = root.querySelector("[data-param='minSev']");
  if (sevSelect) {
    sevSelect.addEventListener("change", () => {
      state.minSeverity = sevSelect.value;
      render();
    });
  }

  const openCheck = root.querySelector("[data-param='onlyOpen']");
  if (openCheck) {
    openCheck.addEventListener("change", () => {
      state.onlyOpen = openCheck.checked;
      render();
    });
  }

  render();
}

/**
 * 14. Lesson 0012 Simulator: DRC Clearance Halos & Clamped Bird's-Eye Minimap
 */
function initDrcMinimapSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    minClearance: 8.0,
    camX: 0.0,
    camY: 12.0,
  };

  function render() {
    const gap = 7.6;
    const violated = gap < state.minClearance;

    const halo = root.querySelector("[data-svg='drc-halo']");
    if (halo) {
      halo.style.display = violated ? "block" : "none";
    }

    const clampedMinX = Math.max(0, Math.min(100, state.camX));
    const clampedMinY = Math.max(0, Math.min(70, state.camY));
    const clampedMaxX = Math.max(clampedMinX, Math.min(100, state.camX + 40));
    const clampedMaxY = Math.max(clampedMinY, Math.min(70, state.camY + 30));

    const camRect = root.querySelector("[data-svg='minimap-cam']");
    if (camRect) {
      camRect.setAttribute("x", 24 + clampedMinX * 2.5);
      camRect.setAttribute("y", 20 + clampedMinY * 2.2);
      camRect.setAttribute("width", (clampedMaxX - clampedMinX) * 2.5);
      camRect.setAttribute("height", (clampedMaxY - clampedMinY) * 2.2);
    }

    const stateEl = root.querySelector("[data-role='drc-state']");
    if (stateEl) {
      stateEl.textContent = `// pad_edge_gap_mm(U1.VCC, U1.SPI_CLK) = 7.6 mm
min_clearance_mm: ${state.minClearance.toFixed(1)},
drc_violations: ${violated ? "1 (7.6 mm < " + state.minClearance.toFixed(1) + " mm)" : "0 (PASS)"},
// Clamped to board_bounds (0, 0)..(100, 70):
minimap_rect: (${clampedMinX.toFixed(0)}, ${clampedMinY.toFixed(0)})..(${clampedMaxX.toFixed(0)}, ${clampedMaxY.toFixed(0)}) mm`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  const clearInput = root.querySelector("[data-param='clearance']");
  if (clearInput) {
    clearInput.addEventListener("input", () => {
      state.minClearance = parseFloat(clearInput.value);
      render();
    });
  }

  const camXInput = root.querySelector("[data-param='camX']");
  if (camXInput) {
    camXInput.addEventListener("input", () => {
      state.camX = parseFloat(camXInput.value);
      render();
    });
  }

  render();
}

/**
 * 15. Lesson 0013 Simulator: Custom LayerSwatch<'a> Widget & Per-Layer Alpha Blending
 */
function initLayerSwatchSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    topVisible: true,
    topOpacity: 0.75,
    bottomVisible: true,
    bottomOpacity: 1.0,
  };

  function render() {
    const topGroup = root.querySelector("[data-svg='swatch-top']");
    if (topGroup) {
      topGroup.style.display = state.topVisible ? "block" : "none";
      topGroup.style.opacity = state.topOpacity;
    }

    const botGroup = root.querySelector("[data-svg='swatch-bottom']");
    if (botGroup) {
      botGroup.style.display = state.bottomVisible ? "block" : "none";
      botGroup.style.opacity = state.bottomOpacity;
    }

    const alphaTop = Math.round(Math.max(0.1, Math.min(1.0, state.topOpacity)) * 255);
    const badgeTop = `${Math.round(state.topOpacity * 100)}%`;

    const stateEl = root.querySelector("[data-role='swatch-state']");
    if (stateEl) {
      stateEl.textContent = `// ui.add(LayerSwatch::new(&mut show_top, &mut top_opacity, ...))
show_top_copper: ${state.topVisible},
top_copper_opacity: ${state.topOpacity.toFixed(2)}, // badge: "${badgeTop}"
// PcbLayer::TopCopper.color_with_opacity(${state.topOpacity.toFixed(2)}):
Color32::from_rgba_unmultiplied(224, 82, 67, ${alphaTop})`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  const topCheck = root.querySelector("[data-param='topVis']");
  if (topCheck) {
    topCheck.addEventListener("change", () => {
      state.topVisible = topCheck.checked;
      render();
    });
  }

  const topSlider = root.querySelector("[data-param='topAlpha']");
  if (topSlider) {
    topSlider.addEventListener("input", () => {
      state.topOpacity = parseFloat(topSlider.value);
      render();
    });
  }

  const botCheck = root.querySelector("[data-param='botVis']");
  if (botCheck) {
    botCheck.addEventListener("change", () => {
      state.bottomVisible = botCheck.checked;
      render();
    });
  }

  const botSlider = root.querySelector("[data-param='botAlpha']");
  if (botSlider) {
    botSlider.addEventListener("input", () => {
      state.bottomOpacity = parseFloat(botSlider.value);
      render();
    });
  }

  render();
}

/**
 * 16. Lesson 0014 Simulator: Multi-Crate L1-L6 Workspace Layering & Manifest Gate
 */
function initWorkspaceLayersSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    egui: false,
    eframe: false,
    rfd: false,
  };

  function render() {
    const forbiddenOrder = ["egui", "eframe", "rfd"];
    const firstViolation = forbiddenOrder.find((dep) => state[dep]) || null;

    const l1Box = root.querySelector("[data-svg='l1-box']");
    const l1Status = root.querySelector("[data-svg='l1-status']");
    if (l1Box && l1Status) {
      if (firstViolation) {
        l1Box.setAttribute("stroke", "#ff5555");
        l1Box.setAttribute("fill", "#2a1418");
        l1Status.setAttribute("fill", "#ff8b6b");
        l1Status.textContent = `ERR: "${firstViolation}" forbidden in L1!`;
      } else {
        l1Box.setAttribute("stroke", "#3b8eea");
        l1Box.setAttribute("fill", "#141a21");
        l1Status.setAttribute("fill", "#3dd68c");
        l1Status.textContent = "PASS: 0 UI crates in L1";
      }
    }

    const extraLines = forbiddenOrder
      .filter((dep) => state[dep])
      .map((dep) => `${dep} = "0.33"`)
      .join("\n");
    const tomlSnippet = `[package]\nname = "pcb-doc"\n\n[dependencies]\nemath = "0.33"${extraLines ? "\n" + extraLines : ""}`;
    const gateResult = firstViolation ? `Err("${firstViolation}")` : "Ok(())";

    const stateEl = root.querySelector("[data-role='layers-state']");
    if (stateEl) {
      stateEl.textContent = `// crates/pcb-doc/Cargo.toml
${tomlSnippet}

// validate_l1_manifest_no_ui_deps(cargo_toml):
=> ${gateResult}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  root.querySelectorAll("[data-dep]").forEach((el) => {
    const dep = el.getAttribute("data-dep");
    el.addEventListener("change", () => {
      state[dep] = el.checked;
      render();
    });
  });

  render();
}

/**
 * 17. Lesson 0015 Simulator: Serde Session Envelope & Typed Error Recovery
 */
function initSerdeSessionSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const scenarios = {
    valid: {
      json: `{
  "schema_version": 1,
  "board_id": "pwr-stage-rev-b",
  "board": {
    "traces": [ /* 6 segments */ ],
    "pads": [ /* 3 pads */ ],
    "review_pins": { "1": { "status": "Open" } }
  }
}`,
      result: `Ok(("pwr-stage-rev-b", BoardModel { .. }))
// ViewState after import:
active_board_id: "pwr-stage-rev-b",
last_io_error: None,`,
    },
    version99: {
      json: `{
  "schema_version": 99,
  "board_id": "pwr-stage-rev-b",
  "board": { /* ... */ }
}`,
      result: `Err(SessionIoError::UnsupportedSchemaVersion {
    found: 99,
    expected: 1,
})
// Active board preserved! UI banner:
last_io_error: Some("unsupported schema_version 99 (expected 1)")`,
    },
    emptyid: {
      json: `{
  "schema_version": 1,
  "board_id": "   ",
  "board": { /* ... */ }
}`,
      result: `Err(SessionIoError::EmptyBoardId)
// Active board preserved! UI banner:
last_io_error: Some("board_id must not be empty")`,
    },
    corrupt: {
      json: `{corrupt`,
      result: `Err(SessionIoError::MalformedJson(
    "key must be a string at line 1 column 2"
))
// Active board preserved! UI banner:
last_io_error: Some("malformed session JSON: ...")`,
    },
  };

  let current = "valid";

  function render() {
    const item = scenarios[current] || scenarios.valid;
    const jsonEl = root.querySelector("[data-role='serde-json-preview']");
    if (jsonEl) {
      jsonEl.textContent = item.json;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(jsonEl);
      }
    }
    const resEl = root.querySelector("[data-role='serde-result-state']");
    if (resEl) {
      resEl.textContent = item.result;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(resEl);
      }
    }
  }

  root.querySelectorAll("[data-scenario]").forEach((btn) => {
    btn.addEventListener("click", () => {
      current = btn.getAttribute("data-scenario");
      render();
    });
  });

  render();
}

/**
 * 18. Lesson 0016 Simulator: Copy-on-Write Arc::make_mut & Arc::ptr_eq Visualizer
 */
function initCowArcSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  let allocCounter = 104;
  const state = {
    baseline: { traces: 100, pads: 101, components: 102, review_pins: 103 },
    working: { traces: 100, pads: 101, components: 102, review_pins: 103 },
    lastOp: "let working = baseline.clone(); // O(1) — 4 Arcs shared",
  };

  function render() {
    const keys = ["traces", "pads", "components", "review_pins"];
    let sharedCount = 0;
    const barsEl = root.querySelector("[data-role='cow-bars']");
    if (barsEl) {
      barsEl.innerHTML = keys
        .map((k) => {
          const shared = state.baseline[k] === state.working[k];
          if (shared) sharedCount += 1;
          const badgeColor = shared ? "#3dd68c" : "#ff8b6b";
          const status = shared
            ? `SHARED (0x${state.working[k].toString(16)})`
            : `DETACHED (base=0x${state.baseline[k].toString(16)}, work=0x${state.working[k].toString(16)})`;
          return `<div style="display:flex; justify-content:space-between; padding:0.42rem 0.55rem; margin-bottom:0.35rem; background:#141a21; border:1px solid ${badgeColor}; border-radius:4px;">
            <span><strong>${k}</strong></span>
            <span style="color:${badgeColor};">${status}</span>
          </div>`;
        })
        .join("");
    } else {
      keys.forEach((k) => {
        if (state.baseline[k] === state.working[k]) sharedCount += 1;
      });
    }

    const stateEl = root.querySelector("[data-role='cow-state']");
    if (stateEl) {
      stateEl.textContent = `// ${state.lastOp}
Arc::ptr_eq(&base.traces, &work.traces): ${state.baseline.traces === state.working.traces},
Arc::ptr_eq(&base.pads, &work.pads): ${state.baseline.pads === state.working.pads},
Arc::ptr_eq(&base.components, &work.components): ${state.baseline.components === state.working.components},
Arc::ptr_eq(&base.review_pins, &work.review_pins): ${state.baseline.review_pins === state.working.review_pins},
working.shared_arc_buffers(&baseline): ${sharedCount} / 4`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  root.querySelectorAll("[data-cow]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const action = btn.getAttribute("data-cow");
      if (action === "pin") {
        if (state.working.review_pins === state.baseline.review_pins) {
          allocCounter += 1;
          state.working.review_pins = allocCounter;
          state.lastOp = "Arc::make_mut(&mut work.review_pins) -> cloned 1 buffer (strong_count == 2)";
        } else {
          state.lastOp = "Arc::make_mut(&mut work.review_pins) -> mutated in-place (strong_count == 1)";
        }
      } else if (action === "pad") {
        if (state.working.pads === state.baseline.pads) {
          allocCounter += 1;
          state.working.pads = allocCounter;
          state.lastOp = "Arc::make_mut(&mut work.pads) -> cloned 1 buffer (strong_count == 2)";
        } else {
          state.lastOp = "Arc::make_mut(&mut work.pads) -> mutated in-place (strong_count == 1)";
        }
      } else if (action === "snapshot") {
        state.baseline = { ...state.working };
        state.lastOp = "baseline = working.clone(); // O(1) — all 4 Arcs shared again";
      } else if (action === "restore") {
        state.working = { ...state.baseline };
        state.lastOp = "working = baseline.clone(); // O(1) instant rollback";
      }
      render();
    });
  });

  render();
}

/**
 * 19. Lesson 0017 Simulator: BoardHistory Undo/Redo & Slider Drag Coalescing
 */
function initUndoCoalesceSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    pinX: 20.0,
    pinCount: 2,
    undoStack: [],
    redoStack: [],
    activeCoalesceKey: null,
    revision: 0,
  };

  function recordEdit(label, beforeSnapshot, coalesceKey) {
    const shouldCoalesce =
      state.activeCoalesceKey !== null &&
      coalesceKey !== null &&
      state.activeCoalesceKey === coalesceKey &&
      state.undoStack.length > 0;

    if (!shouldCoalesce) {
      state.undoStack.push({ label, doc: beforeSnapshot });
      if (state.undoStack.length > 6) {
        state.undoStack.shift();
      }
    }
    state.redoStack = [];
    state.activeCoalesceKey = coalesceKey;
    state.revision += 1;
  }

  function render() {
    const xLabel = root.querySelector("[data-role='pin-x-label']");
    if (xLabel) {
      xLabel.textContent = state.pinX.toFixed(1);
    }
    const slider = root.querySelector("[data-param='pinX']");
    if (slider) {
      slider.value = state.pinX;
    }

    const listEl = root.querySelector("[data-role='undo-stack-list']");
    if (listEl) {
      if (state.undoStack.length === 0 && state.redoStack.length === 0) {
        listEl.innerHTML = `<div style="color:#8b949e;">undo_stack is empty (drag slider or click +Pin)</div>`;
      } else {
        const undoHtml = state.undoStack
          .map(
            (e, i) =>
              `<div style="padding:0.28rem 0.45rem; margin-bottom:0.25rem; background:#141a21; border-left:3px solid #3dd68c;">Undo[${i}]: <strong>${e.label}</strong> (x=${e.doc.pinX.toFixed(1)}, pins=${e.doc.pinCount})</div>`
          )
          .join("");
        const redoHtml = state.redoStack
          .map(
            (e, i) =>
              `<div style="padding:0.28rem 0.45rem; margin-bottom:0.25rem; background:#141a21; border-left:3px solid #3b8eea;">Redo[${i}]: <strong>${e.label}</strong> (x=${e.doc.pinX.toFixed(1)}, pins=${e.doc.pinCount})</div>`
          )
          .join("");
        listEl.innerHTML = undoHtml + redoHtml;
      }
    }

    const stateEl = root.querySelector("[data-role='undo-state']");
    if (stateEl) {
      const keyStr = state.activeCoalesceKey ? `Some("${state.activeCoalesceKey}")` : "None";
      stateEl.textContent = `current: { pin_1.x: ${state.pinX.toFixed(1)} mm, pins: ${state.pinCount} }
active_coalesce_key: ${keyStr}
undo_len(): ${state.undoStack.length} | redo_len(): ${state.redoStack.length}
revision(): ${state.revision}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  const slider = root.querySelector("[data-param='pinX']");
  if (slider) {
    slider.addEventListener("input", () => {
      const before = { pinX: state.pinX, pinCount: state.pinCount };
      state.pinX = parseFloat(slider.value);
      recordEdit("Drag Pin #1 X", before, "drag_pin_1_x");
      render();
    });
  }

  root.querySelectorAll("[data-hist]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const act = btn.getAttribute("data-hist");
      if (act === "commit") {
        state.activeCoalesceKey = null;
      } else if (act === "discrete") {
        const before = { pinX: state.pinX, pinCount: state.pinCount };
        state.pinCount += 1;
        recordEdit("Place Review Pin", before, null);
      } else if (act === "undo" && state.undoStack.length > 0) {
        const entry = state.undoStack.pop();
        state.activeCoalesceKey = null;
        state.redoStack.push({
          label: entry.label,
          doc: { pinX: state.pinX, pinCount: state.pinCount },
        });
        state.pinX = entry.doc.pinX;
        state.pinCount = entry.doc.pinCount;
        state.revision += 1;
      } else if (act === "redo" && state.redoStack.length > 0) {
        const entry = state.redoStack.pop();
        state.activeCoalesceKey = null;
        state.undoStack.push({
          label: entry.label,
          doc: { pinX: state.pinX, pinCount: state.pinCount },
        });
        state.pinX = entry.doc.pinX;
        state.pinCount = entry.doc.pinCount;
        state.revision += 1;
      }
      render();
    });
  });

  render();
}


/**
 * 20. Lesson 0018 Simulator: Unified CommandSpec Registry & panic_hunt Fuzzer
 */
function initCommandPaletteSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    pins: 2,
    undoLen: 0,
    revision: 0,
    query: "",
    lastPrecheck: "Ready — select a CommandSpec or run panic_hunt.",
    lastResult: 'Ok({"pin_count": 2, "pad_count": 3, "revision": 0})',
  };

  const specs = [
    {
      id: "review.addPin",
      label: "Place Review Pin",
      shortcut: "P",
      journal: true,
      enabled: () => ({ ok: true, reason: null }),
      run: () => {
        state.pins += 1;
        state.undoLen += 1;
        state.revision += 1;
        return `Ok({"added_pin_id": ${state.pins}, "pin_count": ${state.pins}})`;
      },
    },
    {
      id: "review.clearPins",
      label: "Clear Review Pins",
      shortcut: null,
      journal: true,
      enabled: () =>
        state.pins > 0
          ? { ok: true, reason: null }
          : { ok: false, reason: "board has no review pins to clear" },
      run: () => {
        state.pins = 0;
        state.undoLen += 1;
        state.revision += 1;
        return `Ok({"cleared": true, "pin_count": 0})`;
      },
    },
    {
      id: "history.undo",
      label: "Undo Last Edit",
      shortcut: "Z",
      journal: false,
      enabled: () =>
        state.undoLen > 0
          ? { ok: true, reason: null }
          : { ok: false, reason: "undo stack is empty" },
      run: () => {
        state.undoLen -= 1;
        state.revision += 1;
        return `Ok({"undone": true, "undo_len": ${state.undoLen}})`;
      },
    },
    {
      id: "session.inspect",
      label: "Inspect Session Telemetry",
      shortcut: "I",
      journal: false,
      enabled: () => ({ ok: true, reason: null }),
      run: () =>
        `Ok({"pin_count": ${state.pins}, "pad_count": 3, "revision": ${state.revision}})`,
    },
  ];

  function render() {
    const listEl = root.querySelector("[data-role='cmd-list']");
    const q = state.query.trim().toLowerCase();
    const filtered = specs.filter(
      (s) =>
        !q ||
        s.id.toLowerCase().includes(q) ||
        s.label.toLowerCase().includes(q)
    );

    if (listEl) {
      listEl.innerHTML = filtered
        .map((s) => {
          const chk = s.enabled();
          const badge = s.shortcut ? `<kbd>${s.shortcut}</kbd>` : "";
          const border = chk.ok ? "#30363d" : "#6e3630";
          const opacity = chk.ok ? "1" : "0.65";
          return `<button type="button" class="egui-btn" data-cmd-id="${s.id}" style="display:flex; justify-content:space-between; align-items:center; border-color:${border}; opacity:${opacity}; text-align:left;">
            <span><strong>${s.label}</strong> <code style="font-size:0.72rem; color:#8b949e;">${s.id}</code></span>
            <span>${badge}</span>
          </button>`;
        })
        .join("");

      listEl.querySelectorAll("[data-cmd-id]").forEach((btn) => {
        btn.addEventListener("click", () => {
          const cmdId = btn.getAttribute("data-cmd-id");
          const spec = specs.find((s) => s.id === cmdId);
          if (!spec) return;
          const chk = spec.enabled();
          if (!chk.ok) {
            state.lastPrecheck = `Precheck FAILED for "${spec.id}":\nenabled(&board, &history) -> Err("${chk.reason}")`;
            state.lastResult = `Err(EngineError::DisabledCommand("${chk.reason}"))`;
          } else {
            state.lastPrecheck = `Precheck OK for "${spec.id}":\njournal=${spec.journal}, enabled=Ok(())`;
            state.lastResult = spec.run();
          }
          render();
        });
      });
    }

    const preEl = root.querySelector("[data-role='cmd-precheck']");
    if (preEl) {
      preEl.textContent = state.lastPrecheck;
    }

    const stateEl = root.querySelector("[data-role='cmd-state']");
    if (stateEl) {
      stateEl.textContent = `// Active state: pins=${state.pins}, undo_len=${state.undoLen}, rev=${state.revision}
// Registered specs: ${specs.length} (COMMAND_PARITY_FLOOR = 5)
${state.lastResult}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  const filterInput = root.querySelector("[data-role='cmd-filter']");
  if (filterInput) {
    filterInput.addEventListener("input", () => {
      state.query = filterInput.value;
      render();
    });
  }

  const fuzzBtn = root.querySelector("[data-cmd-action='fuzz']");
  if (fuzzBtn) {
    fuzzBtn.addEventListener("click", () => {
      state.lastPrecheck =
        'panic_hunt: Tested 5 CommandSpecs x 7 adversarial JSON payloads (null, bool, x_mm=-50.0, comment="   ").\n0 panics, 0 hangs!';
      state.lastResult = `// panic_hunt summary:
// review.addPin({"x_mm": -50.0}) -> Err(BadParams("x_mm and y_mm must be within 0.0..=100.0"))
// unknown.cmd({}) -> Err(UnknownCommand("unknown.cmd"))`;
      render();
    });
  }

  render();
}

/**
 * 21. Lesson 0019 Simulator: Spatial Viewport Culling & PerfBudget
 */
function initViewportCullSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const pads = [
    { x: 20, y: 22, w: 6, h: 4, color: "#e05243" },
    { x: 20, y: 32, w: 6, h: 4, color: "#e05243" },
    { x: 20, y: 48, w: 6, h: 4, color: "#3b8eea" },
    { x: 52, y: 22, w: 6, h: 4, color: "#e05243" },
    { x: 68, y: 35, w: 6, h: 4, color: "#e05243" },
    { x: 82, y: 50, w: 6, h: 4, color: "#3b8eea" },
  ];

  const state = {
    minX: 10,
    width: 42,
    enabled: true,
  };

  function render() {
    const maxX = state.minX + state.width;
    const itemsGroup = root.querySelector("[data-svg='cull-items']");
    const boxEl = root.querySelector("[data-svg='cull-box']");

    if (boxEl) {
      const svgX = 16 + (state.minX / 100) * 288;
      const svgW = (state.width / 100) * 288;
      boxEl.setAttribute("x", svgX.toFixed(1));
      boxEl.setAttribute("width", svgW.toFixed(1));
      boxEl.style.opacity = state.enabled ? "1" : "0.25";
    }

    let visiblePads = 0;
    if (itemsGroup) {
      itemsGroup.innerHTML = pads
        .map((p) => {
          const pMinX = p.x - p.w / 2;
          const pMaxX = p.x + p.w / 2;
          const intersects = !state.enabled || (pMaxX >= state.minX && pMinX <= maxX);
          if (intersects) visiblePads += 1;
          const sx = 16 + ((p.x - p.w / 2) / 100) * 288;
          const sy = 14 + ((p.y - p.h / 2) / 70) * 172;
          const sw = (p.w / 100) * 288;
          const sh = (p.h / 70) * 172;
          const op = intersects ? "1.0" : "0.18";
          const stroke = intersects ? "#ffffff" : "none";
          return `<rect x="${sx.toFixed(1)}" y="${sy.toFixed(1)}" width="${sw.toFixed(1)}" height="${sh.toFixed(1)}" rx="2" fill="${p.color}" opacity="${op}" stroke="${stroke}" stroke-width="1"/>`;
        })
        .join("");
    }

    const culledCount = pads.length - visiblePads;
    const estUs = state.enabled ? 95 + visiblePads * 18 : 95 + pads.length * 42;
    const stateEl = root.querySelector("[data-role='cull-state']");
    if (stateEl) {
      stateEl.textContent = `viewport_mm: Rect::from_min_max(pos2(${state.minX}.0, 0.0), pos2(${maxX}.0, 70.0))
enable_viewport_culling: ${state.enabled}
visible_pads: ${visiblePads} / ${pads.length} (culled: ${culledCount})
check_cull_budget(${estUs} us, max=500 us): Ok(${estUs})`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stateEl);
      }
    }
  }

  root.querySelectorAll("[data-cull]").forEach((el) => {
    el.addEventListener("input", () => {
      const k = el.getAttribute("data-cull");
      if (k === "minX") state.minX = parseInt(el.value, 10);
      if (k === "width") state.width = parseInt(el.value, 10);
      if (k === "enabled") state.enabled = el.checked;
      render();
    });
  });

  render();
}

/**
 * 22. Lesson 0020 Simulator: Background mpsc File Import & ProgressBar
 */
function initAsyncImportSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    queue: [],
    jobState: "ImportJobState::Idle",
    fraction: 0.0,
    phaseLabel: "Idle",
    repaintRequested: false,
  };

  function render() {
    const barEl = root.querySelector("[data-role='import-bar']");
    const lblEl = root.querySelector("[data-role='import-bar-label']");
    const qEl = root.querySelector("[data-role='import-queue']");
    const stEl = root.querySelector("[data-role='import-state']");

    if (barEl) barEl.style.width = `${Math.round(state.fraction * 100)}%`;
    if (lblEl) lblEl.textContent = `${state.phaseLabel} (${Math.round(state.fraction * 100)}%)`;
    if (qEl) {
      qEl.innerHTML =
        state.queue.length === 0
          ? `<span style="color:#8b949e;">mpsc queue empty (TryRecvError::Empty / Disconnected)</span>`
          : state.queue
              .map(
                (m, i) =>
                  `<div style="color:#7fe5f0;">queued[${i}]: ${m.summary}</div>`
              )
              .join("");
    }
    if (stEl) {
      stEl.textContent = `import_state: ${state.jobState}
status_badge(): "${state.phaseLabel}"
ctx.request_repaint(): ${state.repaintRequested}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stEl);
      }
    }
  }

  root.querySelectorAll("[data-import]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const act = btn.getAttribute("data-import");
      if (act === "start-valid") {
        state.queue = [
          {
            summary: 'Progress(1/2, "Validating envelope")',
            fraction: 0.5,
            label: "Importing: Validating envelope (50%)",
            job: 'ImportJobState::Running { fraction: 0.5, phase: "Validating envelope" }',
            repaint: true,
          },
          {
            summary: 'Progress(2/2, "Loading copper layers")',
            fraction: 1.0,
            label: "Importing: Loading copper layers (100%)",
            job: 'ImportJobState::Running { fraction: 1.0, phase: "Loading copper layers" }',
            repaint: true,
          },
          {
            summary: 'Completed { board_id: "pwr-stage-rev-b" }',
            fraction: 1.0,
            label: "Import Complete: pwr-stage-rev-b",
            job: 'ImportJobState::Succeeded { board_id: "pwr-stage-rev-b" }',
            repaint: false,
          },
        ];
        state.fraction = 0.05;
        state.phaseLabel = "Importing: Spawning worker (5%)";
        state.jobState = 'ImportJobState::Running { fraction: 0.05, phase: "Spawning worker" }';
        state.repaintRequested = true;
      } else if (act === "start-corrupt") {
        state.queue = [
          {
            summary: 'Progress(1/2, "Validating envelope")',
            fraction: 0.5,
            label: "Importing: Validating envelope (50%)",
            job: 'ImportJobState::Running { fraction: 0.5, phase: "Validating envelope" }',
            repaint: true,
          },
          {
            summary: 'Failed(MalformedJson("key must be a string"))',
            fraction: 0.0,
            label: "Import Failed: malformed session JSON",
            job: 'ImportJobState::Failed("malformed session JSON")',
            repaint: false,
          },
        ];
        state.fraction = 0.05;
        state.phaseLabel = "Importing: Spawning worker (5%)";
        state.jobState = 'ImportJobState::Running { fraction: 0.05, phase: "Spawning worker" }';
        state.repaintRequested = true;
      } else if (act === "step") {
        if (state.queue.length > 0) {
          const msg = state.queue.shift();
          state.fraction = msg.fraction;
          state.phaseLabel = msg.label;
          state.jobState = msg.job;
          state.repaintRequested = msg.repaint;
        }
      } else if (act === "cancel") {
        state.queue = [];
        state.fraction = 0.0;
        state.phaseLabel = "Idle";
        state.jobState = "ImportJobState::Idle";
        state.repaintRequested = false;
      }
      render();
    });
  });

  render();
}

/**
 * 23. Lesson 0021 Simulator: Offscreen Snapshot Digest (gpu_lock) & wasm32 Gate
 */
function initSnapshotWasmSimulator(containerId) {
  const root = document.getElementById(containerId);
  if (!root) return;

  const state = {
    top: true,
    bot: true,
    silk: true,
    rats: false,
    baselineHash: null,
    target: "native-desktop",
  };

  function computeHash() {
    const mask =
      (state.top ? 1 : 0) |
      (state.bot ? 2 : 0) |
      (state.silk ? 4 : 0) |
      (state.rats ? 8 : 0);
    const pads = (state.top ? 2 : 0) + (state.bot ? 1 : 0);
    const traces =
      (state.top ? 2 : 0) +
      (state.bot ? 2 : 0) +
      (state.silk ? 1 : 0) +
      (state.rats ? 1 : 0);
    const hashNum = ((0xcbf29ce4 ^ mask) * 0x1000193 + pads * 31 + traces * 17) >>> 0;
    return {
      mask,
      pads,
      traces,
      hex: "0x" + hashNum.toString(16).padStart(8, "0"),
    };
  }

  state.baselineHash = computeHash().hex;

  function render() {
    const cur = computeHash();
    const matches = cur.hex === state.baselineHash;
    const diffEl = root.querySelector("[data-role='snap-diff']");
    if (diffEl) {
      diffEl.innerHTML = `<div style="margin-bottom:0.35rem;"><code>let _gpu = gpu_lock();</code> <span style="color:#3dd68c;">(Acquired MutexGuard)</span></div>
      <div>Baseline Digest: <code>${state.baselineHash}</code></div>
      <div>Current Digest:  <code>${cur.hex}</code></div>
      <div style="margin-top:0.35rem; font-weight:600; color:${matches ? "#3dd68c" : "#ff6b5b"};">
        Snapshot Status: ${matches ? "MATCH (Visual Parity Verified)" : "MISMATCH (Regression Caught!)"}
      </div>`;
    }

    const stEl = root.querySelector("[data-role='snap-state']");
    if (stEl) {
      stEl.textContent = `RuntimeTarget::current(): "${state.target}"
FrameSnapshotDigest {
  visible_layer_mask: 0b${cur.mask.toString(2).padStart(4, "0")},
  visible_pad_count: ${cur.pads},
  visible_trace_count: ${cur.traces},
  fnv1a_hash: ${cur.hex},
}`;
      if (window.Prism && window.Prism.highlightElement) {
        window.Prism.highlightElement(stEl);
      }
    }
  }

  root.querySelectorAll("[data-snap]").forEach((cb) => {
    cb.addEventListener("change", () => {
      const k = cb.getAttribute("data-snap");
      state[k] = cb.checked;
      render();
    });
  });

  root.querySelectorAll("[data-snap-act]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const act = btn.getAttribute("data-snap-act");
      if (act === "bless") {
        state.baselineHash = computeHash().hex;
      } else if (act === "target") {
        state.target =
          state.target === "native-desktop"
            ? "wasm32-unknown-unknown"
            : "native-desktop";
      }
      render();
    });
  });

  render();
}
