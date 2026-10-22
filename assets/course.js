/* ==========================================================================
   Learn egui — Reusable Interactive Course Components
   Shared across ./lessons/*.html
   ========================================================================== */

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
    // Update Rust struct state inspector
    const stateEl = root.querySelector("[data-role='rust-state']");
    if (stateEl) {
      stateEl.innerHTML = `<span class="kw">struct</span> <span class="ty">PcbReviewerApp</span> {
  show_top_copper: <span class="mut">${state.showTopCopper}</span>,
  show_bottom_copper: <span class="mut">${state.showBottomCopper}</span>,
  show_silkscreen: <span class="mut">${state.showSilkscreen}</span>,
  show_ratsnest: <span class="mut">${state.showRatsnest}</span>,
}`;
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
      state.showBottomCopper = true;
      state.showSilkscreen = false;
      state.showRatsnest = false;
      state.frameCount += 1;
      state.lastEvent = `ui.button("Copper Only").clicked() == true -> hid Silk & Ratsnest`;
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
