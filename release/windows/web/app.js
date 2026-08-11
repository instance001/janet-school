const UI_STATE = {
  selectedAction: "sync_state",
  selectedTeacherBackend: null,
  sessionName: "GUI Session",
  selectedRunMode: null,
  selectedCurriculumSizeHint: null,
  selectedModelPath: null,
  selectedEndpoint: null,
  selectedSessionsDir: null,
  selectedAggregatedDir: null,
  selectedCurriculumDomainId: null,
  selectedCurriculumItemId: null,
  selectedSkillIds: null,
  comparePrimaryRunId: null,
  compareSecondaryRunId: null,
  compareShowChangedOnly: true,
  compareShowRefusalsOnly: false,
  compareShowAnomaliesOnly: false,
  compareSkillShiftOnly: false,
  compareDomainFilter: "all",
  compareItemTypeFilter: "all",
  compareExportScope: "visible",
  actionRunning: false,
  activeJobId: null,
  activeJobState: null,
  statusMessage: "",
  statusTone: "neutral",
  autoRefreshHandle: null,
  bridgeStatusHandle: null,
  bridgeStatus: null,
  lastCompareExport: null,
  aboutExpanded: false,
  splashDismissed: false,
  splashTimerHandle: null,
  splashListenersBound: false,
  tooltipBound: false,
};

function captureRenderState() {
  const openDetails = {};
  document.querySelectorAll("details[data-persist-key]").forEach((detail) => {
    openDetails[detail.dataset.persistKey] = detail.open;
  });

  const scrollPositions = {};
  document.querySelectorAll("[data-preserve-scroll]").forEach((element) => {
    if (!element.id) return;
    scrollPositions[element.id] = element.scrollTop;
  });

  return {
    openDetails,
    scrollPositions,
    windowScrollX: window.scrollX,
    windowScrollY: window.scrollY,
  };
}

function restoreRenderState(snapshot) {
  if (!snapshot) return;

  document.querySelectorAll("details[data-persist-key]").forEach((detail) => {
    const key = detail.dataset.persistKey;
    if (!key) return;
    if (snapshot.openDetails[key]) {
      detail.open = true;
    }
  });

  document.querySelectorAll("[data-preserve-scroll]").forEach((element) => {
    if (!element.id) return;
    const nextScrollTop = snapshot.scrollPositions[element.id];
    if (typeof nextScrollTop === "number") {
      element.scrollTop = nextScrollTop;
    }
  });

  window.scrollTo(snapshot.windowScrollX ?? window.scrollX, snapshot.windowScrollY ?? window.scrollY);
}

async function loadGuiState(options = {}) {
  const { silent = false } = options;

  try {
    const state = window.__JANET_BRIDGE__?.loadState
      ? await window.__JANET_BRIDGE__.loadState()
      : await loadGuiStateFromFile();
    window.__JANET_SCHOOL__ = state;
    initializeSplash(state);
    initializeTooltips();
    if (silent && isInteractiveEditing()) {
      return;
    }
    render(state);
  } catch (error) {
    if (!silent) {
      revealMainUi();
      renderError(error);
    } else {
      setStatus(`Auto-refresh failed: ${error.message}`, "warning");
      syncRenderedStatus();
    }
  }
}

async function loadGuiStateFromFile() {
  const response = await fetch("./gui-state.json", { cache: "no-store" });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.json();
}

function render(state) {
  const renderSnapshot = captureRenderState();
  const latest = state.latest_session;
  const liveJob = UI_STATE.bridgeStatus?.active_job ?? null;
  const liveProgress = liveJob?.progress ?? null;
  document.getElementById("hero-meta").innerHTML = `
    <span>${state.app_name} ${state.version}</span>
    <span>${state.backend_mode}</span>
    <span>teacher: ${state.teacher_backend}</span>
    <span>default: ${state.configured_teacher_backend}</span>
    <span>bridge: ${state.control_surface.bridge_mode}</span>
    <span>generated: ${state.generated_at}</span>
  `;

  renderControls(state);
  ensureAutoRefresh(state);
  ensureBridgeStatusPolling(state);
  document.getElementById("live-job-detail").innerHTML = renderLiveJobDetail(liveJob);

  if (!latest) {
    document.getElementById("setup-detail").textContent = "No sessions available yet.";
    document.getElementById("curriculum-detail").textContent =
      "Run a session from the CLI to populate this shell.";
    document.getElementById("run-detail").textContent = "No run stats available.";
    document.getElementById("telemetry-detail").textContent =
      "No telemetry snapshot available.";
    document.getElementById("analysis-detail").textContent =
      "No analysis snapshot available.";
    document.getElementById("export-detail").textContent =
      "No export artifacts available.";
    document.getElementById("about-detail").innerHTML = renderAboutDetail(state);
    bindAboutDetail();
    document.getElementById("recent-sessions").innerHTML = "<p>No recent sessions.</p>";
    document.getElementById("compare-sessions").innerHTML = "<p>No sessions available to compare yet.</p>";
    publishChattyCogStatus(state);
    restoreRenderState(renderSnapshot);
    return;
  }

  document.getElementById("setup-detail").innerHTML = `
    ${renderSetupDetail(state, latest)}
  `;
  bindSetupDetail(state);

  document.getElementById("curriculum-detail").innerHTML = `
    ${renderCurriculumDetail(latest)}
  `;
  bindCurriculumInspector(latest);

  document.getElementById("run-detail").innerHTML = `
    ${renderRunDetail(latest, liveJob)}
  `;
  const analysis = latest.analysis_snapshot;
  const teacher = latest.teacher_snapshot;
  const teacherRationale = teacher?.rationale
    ? truncateText(teacher.rationale, 180)
    : teacher?.error ?? "n/a";
  const teacherLine = teacher
    ? `
    <p>Teacher latency / tokens:
      <strong>${teacher.latency_ms} ms / ${teacher.token_counts?.input_tokens ?? 0} in / ${teacher.token_counts?.output_tokens ?? 0} out</strong>
    </p>
    <p>Teacher runtime:
      <strong>${teacher.endpoint_ready ? "ready" : "not ready"}${teacher.launched_runtime ? ", launched now" : ""}</strong>
    </p>
    <p>Teacher domains:
      <strong>${teacher.selected_domain_ids.length ? teacher.selected_domain_ids.join(", ") : "n/a"}</strong>
    </p>
    <p>Teacher rationale:
      <strong>${teacherRationale}</strong>
    </p>
  `
    : "<p>Teacher snapshot unavailable.</p>";
  document.getElementById("telemetry-detail").innerHTML = `
    <div class="telemetry-panel">
      <div class="telemetry-group">
        <h3>Session Totals</h3>
        <p>Memory reads: <strong>${latest.memory_stats.memory_reads ?? 0}</strong></p>
        <p>Refusals: <strong>${latest.refusal_stats.refusal_count ?? 0}</strong></p>
        <p>Anomaly flags: <strong>${latest.anomaly_stats.anomaly_flag_count ?? 0}</strong></p>
        <p>Confirmed / boundary / emergent:
          <strong>${analysis?.confirmed_count ?? 0} / ${analysis?.boundary_count ?? 0} / ${analysis?.emergent_count ?? 0}</strong>
        </p>
        ${teacherLine}
      </div>
      <div class="telemetry-group">
        <h3>Recent Evidence Trail</h3>
        ${renderTelemetryPreview(latest.telemetry_preview ?? [])}
      </div>
    </div>
  `;
  document.getElementById("analysis-detail").innerHTML = renderAnalysisDetail(state, latest);
  document.getElementById("export-detail").innerHTML = renderExportDetail(latest);
  bindExportDetail(latest);
  document.getElementById("about-detail").innerHTML = renderAboutDetail(state);
  bindAboutDetail();
  document.getElementById("recent-sessions").innerHTML = renderRecentSessions(state.recent_sessions);
  document.getElementById("compare-sessions").innerHTML = renderCompareSessions(state.recent_sessions);
  bindCompareSessions(state.recent_sessions);
  publishChattyCogStatus(state);
  restoreRenderState(renderSnapshot);
}

function publishChattyCogStatus(state) {
  if (!window.chattyCogBridge?.available || typeof window.chattyCogBridge.updateStatus !== "function") {
    return;
  }

  try {
    const payload = buildChattyCogStatusPayload(state);
    if (!payload) return;
    window.chattyCogBridge.updateStatus(payload);
  } catch (error) {
    console.warn("ChattyCog bridge status update failed", error);
  }
}

function buildChattyCogStatusPayload(state) {
  if (!state || typeof state !== "object") return null;

  const latest = state.latest_session ?? null;
  const totalSkills = state.skill_snapshot?.entries?.length ?? 0;
  const approvedSkills = (state.skill_snapshot?.entries ?? []).filter((entry) => entry.approved).length;
  const warnings = state.setup_snapshot?.warnings ?? [];

  const summary = latest
    ? `Janet School latest run ${latest.run_id.slice(0, 8)} uses ${latest.teacher_backend_id}, completed ${numericStat(latest.interaction_stats, "total_items", 0)} items, logged ${numericStat(latest.refusal_stats, "refusal_count", 0)} refusals, and surfaced ${numericStat(latest.anomaly_stats, "anomaly_flag_count", 0)} anomaly flags.`
    : `Janet School is ready in ${state.backend_mode} mode with ${approvedSkills}/${totalSkills} approved deterministic skills and no completed sessions yet.`;

  const snapshotLines = [
    "# Janet School Snapshot",
    "",
    `- Version: ${state.version}`,
    `- Backend mode: ${state.backend_mode}`,
    `- Active teacher backend: ${state.teacher_backend}`,
    `- Configured teacher backend: ${state.configured_teacher_backend}`,
    `- Skill approvals: ${approvedSkills}/${totalSkills}`,
    `- Runtime endpoint: ${state.setup_snapshot?.endpoint ?? "n/a"}`,
    `- Runtime ready: ${state.setup_snapshot?.endpoint_ready ? "yes" : "no"}`,
    `- Runtime binary present: ${state.setup_snapshot?.server_binary_exists ? "yes" : "no"}`,
    `- Teacher model present: ${state.setup_snapshot?.model_path_exists ? "yes" : "no"}`,
  ];

  if (warnings.length) {
    snapshotLines.push(`- Setup warnings: ${warnings.join(" | ")}`);
  }

  if (latest) {
    snapshotLines.push("");
    snapshotLines.push("## Latest Session");
    snapshotLines.push(`- Run id: ${latest.run_id}`);
    snapshotLines.push(`- Completed at: ${latest.completed_at ?? "in progress or not recorded"}`);
    snapshotLines.push(`- Skill profile: ${formatSkillRunProfile(latest.skill_run_snapshot)}`);
    snapshotLines.push(`- Accuracy: ${formatAccuracy(latest)}`);
    snapshotLines.push(`- Refusals: ${numericStat(latest.refusal_stats, "refusal_count", 0)}`);
    snapshotLines.push(`- Anomaly flags: ${numericStat(latest.anomaly_stats, "anomaly_flag_count", 0)}`);
    snapshotLines.push(`- Confirmed/boundary/emergent: ${latest.analysis_snapshot?.confirmed_count ?? 0}/${latest.analysis_snapshot?.boundary_count ?? 0}/${latest.analysis_snapshot?.emergent_count ?? 0}`);

    if (latest.teacher_snapshot) {
      snapshotLines.push("");
      snapshotLines.push("## Teacher Snapshot");
      snapshotLines.push(`- Teacher latency: ${latest.teacher_snapshot.latency_ms} ms`);
      snapshotLines.push(`- Teacher endpoint ready: ${latest.teacher_snapshot.endpoint_ready ? "yes" : "no"}`);
      snapshotLines.push(`- Teacher launched runtime: ${latest.teacher_snapshot.launched_runtime ? "yes" : "no"}`);
      snapshotLines.push(`- Teacher domains: ${latest.teacher_snapshot.selected_domain_ids.length ? latest.teacher_snapshot.selected_domain_ids.join(", ") : "n/a"}`);
    }

    if (latest.artifacts?.length) {
      snapshotLines.push("");
      snapshotLines.push("## Artifacts");
      latest.artifacts.slice(0, 6).forEach((artifact) => {
        snapshotLines.push(`- ${artifact.label}: ${artifact.relative_path}`);
      });
    }
  }

  return {
    module_id: "janet_school",
    event_type: "suspend_rundown",
    summary,
    snapshot: snapshotLines.join("\n"),
    tags: latest
      ? ["janet_school", "research", "mcm", "telemetry", "latest_session"]
      : ["janet_school", "research", "mcm", "idle"],
    payload: {
      app_name: state.app_name,
      version: state.version,
      backend_mode: state.backend_mode,
      teacher_backend: state.teacher_backend,
      configured_teacher_backend: state.configured_teacher_backend,
      approved_skill_count: approvedSkills,
      total_skill_count: totalSkills,
      has_latest_session: Boolean(latest),
      latest_run_id: latest?.run_id ?? null,
      latest_completed_at: latest?.completed_at ?? null,
      latest_accuracy: latest ? formatAccuracy(latest) : null,
      latest_refusal_count: latest ? numericStat(latest.refusal_stats, "refusal_count", 0) : 0,
      latest_anomaly_flag_count: latest ? numericStat(latest.anomaly_stats, "anomaly_flag_count", 0) : 0,
    },
    updated_at_unix_ms: Date.now(),
  };
}

function renderControls(state) {
  const bridgeReady = Boolean(window.__JANET_BRIDGE__?.runGuiAction);
  const teacherBackendDefault = normalizeTeacherBackend(
    UI_STATE.selectedTeacherBackend || state.configured_teacher_backend || state.teacher_backend || "mock",
  );
  const detail = document.getElementById("control-detail");
  const recentJobs = UI_STATE.bridgeStatus?.recent_jobs ?? [];
  const activeBridgeJob = UI_STATE.bridgeStatus?.active_job ?? null;
  const activeBridgeState = activeBridgeJob?.state ?? UI_STATE.activeJobState ?? null;
  const hasActiveJob = Boolean(activeBridgeJob || UI_STATE.activeJobId);
  ensureSelectedSkills(state.skill_snapshot);
  const selectedSkillIds = UI_STATE.selectedSkillIds ?? [];
  const totalSkills = state.skill_snapshot?.entries?.length ?? 0;
  const dirtySkillSelection = hasSkillSelectionChanges(state.skill_snapshot);
  const activeJobLine = UI_STATE.activeJobId
    ? `<p class="control-note">Active job: <strong>${UI_STATE.activeJobId.slice(0, 8)} / ${UI_STATE.activeJobState ?? "queued"}</strong></p>`
    : "";
  const recentJobsMarkup = recentJobs.length
    ? `
      <div class="bridge-jobs">
        <p class="control-note">Recent bridge jobs</p>
        <div class="bridge-job-list">
          ${recentJobs
            .map(
              (job) => `
                <article class="bridge-job bridge-job--${job.state}">
                  <p><strong>${job.action}</strong> ${job.job_id.slice(0, 8)}</p>
                  <p>State: ${job.state}</p>
                  <p>Backend: ${job.teacher_backend ?? "n/a"}</p>
                  <p>Session: ${job.session_name ?? "n/a"}</p>
                  <p>Progress: ${formatJobProgress(job.progress)}</p>
                  <p>${job.result_summary ?? job.error ?? "waiting for host update"}</p>
                </article>
              `,
            )
            .join("")}
        </div>
      </div>
    `
    : `
      <div class="bridge-jobs">
        <p class="control-note">Recent bridge jobs</p>
        <p class="control-note">No bridge jobs recorded in this host session yet.</p>
      </div>
    `;

  detail.innerHTML = `
    <div class="control-stack">
      <label class="control-row">
        <span>Teacher backend</span>
        <select id="teacher-backend-select" data-tooltip="Choose which teacher backend guides curriculum and session generation. Start with mock for the safest local workflow." ${UI_STATE.actionRunning ? "disabled" : ""}>
          <option value="mock"${teacherBackendDefault === "mock" ? " selected" : ""}>mock</option>
          <option value="local-llm"${teacherBackendDefault === "local-llm" ? " selected" : ""}>local-llm</option>
        </select>
      </label>
      <label class="control-row">
        <span>Session name</span>
        <input id="session-name-input" type="text" data-tooltip="Name this run so it is easier to find later in recent sessions, exports, and compare views." value="${escapeHtml(UI_STATE.sessionName)}" ${UI_STATE.actionRunning ? "disabled" : ""} />
      </label>
      <div class="button-row">
        ${state.control_surface.actions
          .map(
            (action) => `
              <button
                class="action-button${UI_STATE.selectedAction === action.action_id ? " is-selected" : ""}"
                data-action-id="${action.action_id}"
                data-tooltip="${escapeHtml(action.description)}"
                ${isActionDisabled(action.action_id, UI_STATE.actionRunning, hasActiveJob, activeBridgeState) ? "disabled" : ""}
              >
                ${action.label}
              </button>
            `,
          )
          .join("")}
      </div>
      <p class="control-note">
        ${bridgeReady
          ? "Native bridge detected. Actions run as background jobs owned by the Rust host."
          : "No native bridge is attached in this static shell. Buttons will update the CLI command preview instead."}
      </p>
      <p class="control-note">Bridge mode: <strong>${state.control_surface.bridge_mode}</strong></p>
      <p class="control-note">Auto-refresh: <strong>${bridgeReady ? "every 5 seconds" : "disabled"}</strong></p>
      ${activeJobLine}
      <pre class="command-preview" id="command-preview"></pre>
      <p class="control-status control-status--${UI_STATE.statusTone}" id="command-status">${escapeHtml(UI_STATE.statusMessage)}</p>
      <div class="skill-approval-panel">
        <div class="skill-approval-header">
          <p class="control-note">MCM skill approvals</p>
          <p class="control-note">Selected: <strong>${selectedSkillIds.length} / ${totalSkills}</strong></p>
        </div>
        <div class="button-row">
          <button class="action-button" id="skills-select-all" data-tooltip="Approve every listed deterministic skill for upcoming runs."${UI_STATE.actionRunning || hasActiveJob ? " disabled" : ""}>Select All</button>
          <button class="action-button" id="skills-deselect-all" data-tooltip="Turn off every listed skill so you can run memory-only or rebuild a smaller skill set."${UI_STATE.actionRunning || hasActiveJob ? " disabled" : ""}>Deselect All</button>
          <button class="action-button${UI_STATE.selectedAction === "update_skill_approvals" ? " is-selected" : ""}" id="skills-confirm" data-tooltip="Save the current skill selection into the approvals config used by future runs."${UI_STATE.actionRunning || hasActiveJob || !dirtySkillSelection ? " disabled" : ""}>Confirm Skills</button>
        </div>
        <p class="control-note">Use this to run memory-only, single-skill, or grouped-skill sessions for finer triangulation.</p>
        <div class="skill-list" id="skill-list" data-preserve-scroll>
          ${state.skill_snapshot.entries
            .map(
              (skill) => `
                <label class="skill-entry">
                  <input
                    type="checkbox"
                    data-skill-id="${escapeHtml(skill.skill_id)}"
                    data-tooltip="${escapeHtml(`Toggle ${skill.skill_id}. ${skill.description}`)}"
                    ${selectedSkillIds.includes(skill.skill_id) ? "checked" : ""}
                    ${UI_STATE.actionRunning || hasActiveJob ? "disabled" : ""}
                  />
                  <span class="skill-entry-copy">
                    <strong>${escapeHtml(skill.skill_id)}</strong>
                    <span>${escapeHtml(skill.description)}</span>
                  </span>
                </label>
              `,
            )
            .join("")}
        </div>
      </div>
      ${recentJobsMarkup}
    </div>
  `;

  const sessionInput = document.getElementById("session-name-input");
  const backendSelect = document.getElementById("teacher-backend-select");
  const buttons = detail.querySelectorAll("[data-action-id]");
  const preview = document.getElementById("command-preview");
  const skillCheckboxes = detail.querySelectorAll("[data-skill-id]");
  const selectAllButton = document.getElementById("skills-select-all");
  const deselectAllButton = document.getElementById("skills-deselect-all");
  const confirmSkillsButton = document.getElementById("skills-confirm");

  const updatePreview = (actionId) => {
    const action = state.control_surface.actions.find((entry) => entry.action_id === actionId);
    if (!action) return;
    UI_STATE.selectedAction = actionId;
    preview.textContent = buildCommandPreview(
      action,
      backendSelect.value,
      sessionInput.value || "GUI Session",
    );
    if (!UI_STATE.statusMessage) {
      setStatus(action.description, "neutral");
      syncRenderedStatus();
    }
  };

  buttons.forEach((button) => {
    const actionId = button.dataset.actionId;
    button.addEventListener("click", async () => {
      updatePreview(actionId);
      renderControls(state);
      if (!bridgeReady) return;

      try {
        const request = {
          action: actionId,
          teacher_backend: backendSelect.value,
          session_name: sessionInput.value || "GUI Session",
          run_mode: UI_STATE.selectedRunMode,
          curriculum_size_hint: UI_STATE.selectedCurriculumSizeHint,
          model_path: UI_STATE.selectedModelPath,
          endpoint: UI_STATE.selectedEndpoint,
          sessions_dir: UI_STATE.selectedSessionsDir,
          aggregated_dir: UI_STATE.selectedAggregatedDir,
        };
        if (actionId === "update_skill_approvals") {
          request.selected_skill_ids = [...(UI_STATE.selectedSkillIds ?? [])];
        }
        const accepted = await window.__JANET_BRIDGE__.runGuiAction({
          ...request,
        });
        UI_STATE.actionRunning = !["stop_run", "pause_run", "resume_run"].includes(actionId);
        UI_STATE.activeJobId = accepted.job.job_id;
        UI_STATE.activeJobState = accepted.job.state;
        setStatus(
          actionId === "stop_run"
            ? `Stop requested for host job ${accepted.job.job_id.slice(0, 8)}.`
            : actionId === "pause_run"
              ? `Pause requested for host job ${accepted.job.job_id.slice(0, 8)}.`
              : actionId === "resume_run"
                ? `Resume requested for host job ${accepted.job.job_id.slice(0, 8)}.`
                : actionId === "update_skill_approvals"
                  ? `Skill approvals saved through host job ${accepted.job.job_id.slice(0, 8)}.`
            : `Accepted ${actionId}. Waiting for host job ${accepted.job.job_id.slice(0, 8)}.`,
          ["stop_run", "pause_run"].includes(actionId) ? "warning" : actionId === "update_skill_approvals" ? "success" : "running",
        );
        renderControls(state);
      } catch (error) {
        UI_STATE.actionRunning = false;
        UI_STATE.activeJobId = null;
        UI_STATE.activeJobState = null;
        setStatus(`Bridge action failed: ${error.message}`, "error");
        renderControls(state);
      }
    });
  });

  if (!UI_STATE.selectedAction) {
    UI_STATE.selectedAction = "sync_state";
  }
  updatePreview(UI_STATE.selectedAction);

  backendSelect.addEventListener("change", () => {
    UI_STATE.selectedTeacherBackend = backendSelect.value;
    updatePreview(UI_STATE.selectedAction);
  });
  sessionInput.addEventListener("input", () => {
    UI_STATE.sessionName = sessionInput.value;
    updatePreview(UI_STATE.selectedAction);
  });

  skillCheckboxes.forEach((checkbox) => {
    checkbox.addEventListener("change", () => {
      const skillId = checkbox.dataset.skillId;
      const next = new Set(UI_STATE.selectedSkillIds ?? []);
      if (checkbox.checked) {
        next.add(skillId);
      } else {
        next.delete(skillId);
      }
      UI_STATE.selectedSkillIds = [...next];
      renderControls(state);
    });
  });

  selectAllButton?.addEventListener("click", () => {
    UI_STATE.selectedSkillIds = state.skill_snapshot.entries.map((entry) => entry.skill_id);
    renderControls(state);
  });

  deselectAllButton?.addEventListener("click", () => {
    UI_STATE.selectedSkillIds = [];
    renderControls(state);
  });

  confirmSkillsButton?.addEventListener("click", async () => {
    UI_STATE.selectedAction = "update_skill_approvals";
    updatePreview("update_skill_approvals");
    renderControls(state);
    if (!bridgeReady || !dirtySkillSelection) return;

    try {
      const accepted = await window.__JANET_BRIDGE__.runGuiAction({
        action: "update_skill_approvals",
        selected_skill_ids: [...(UI_STATE.selectedSkillIds ?? [])],
      });
      UI_STATE.actionRunning = false;
      UI_STATE.activeJobId = accepted.job.job_id;
      UI_STATE.activeJobState = accepted.job.state;
      setStatus(`Skill approvals saved through host job ${accepted.job.job_id.slice(0, 8)}.`, "success");
      await loadGuiState({ silent: true });
    } catch (error) {
      setStatus(`Skill approval update failed: ${error.message}`, "error");
      syncRenderedStatus();
    }
  });
}

function ensureAutoRefresh(state) {
  const bridgeReady = Boolean(window.__JANET_BRIDGE__?.loadState);
  if (!bridgeReady) {
    if (UI_STATE.autoRefreshHandle) {
      clearInterval(UI_STATE.autoRefreshHandle);
      UI_STATE.autoRefreshHandle = null;
    }
    return;
  }

  if (UI_STATE.autoRefreshHandle) return;

  UI_STATE.autoRefreshHandle = window.setInterval(() => {
    if (UI_STATE.actionRunning || isInteractiveEditing()) return;
    loadGuiState({ silent: true });
  }, 5000);
}

function ensureBridgeStatusPolling(state) {
  const bridgeReady = Boolean(window.__JANET_BRIDGE__?.loadBridgeStatus);
  if (!bridgeReady) {
    if (UI_STATE.bridgeStatusHandle) {
      clearInterval(UI_STATE.bridgeStatusHandle);
      UI_STATE.bridgeStatusHandle = null;
    }
    return;
  }

  if (UI_STATE.bridgeStatusHandle) return;

  UI_STATE.bridgeStatusHandle = window.setInterval(async () => {
    try {
      const status = await window.__JANET_BRIDGE__.loadBridgeStatus();
      UI_STATE.bridgeStatus = status;
      if (!UI_STATE.actionRunning && !UI_STATE.activeJobId) {
        if (isInteractiveEditing()) return;
        render(state);
        return;
      }
      const job = status.recent_jobs.find((entry) => entry.job_id === UI_STATE.activeJobId)
        || status.active_job;

      if (!job) return;

      UI_STATE.activeJobState = job.state;

      if (["queued", "running", "cancelling", "pausing", "paused"].includes(job.state)) {
        setStatus(
          `${job.action} ${job.state}. ${job.result_summary ?? "Host bridge is still processing."}`,
          ["cancelling", "pausing", "paused"].includes(job.state) ? "warning" : "running",
        );
        if (isInteractiveEditing()) {
          syncRenderedStatus();
          return;
        }
        render(state);
        return;
      }

      UI_STATE.actionRunning = false;
      UI_STATE.activeJobId = null;

      if (job.state === "completed") {
        setStatus(job.result_summary ?? `${job.action} completed.`, "success");
      } else if (job.state === "stopped") {
        setStatus(job.result_summary ?? `${job.action} stopped.`, "warning");
      } else {
        setStatus(job.error ?? `${job.action} failed.`, "error");
      }

      await loadGuiState({ silent: true });
    } catch (error) {
      setStatus(`Bridge status polling failed: ${error.message}`, "warning");
      syncRenderedStatus();
    }
  }, 1500);

  window.__JANET_BRIDGE__.loadBridgeStatus()
    .then((status) => {
      UI_STATE.bridgeStatus = status;
      if (isInteractiveEditing()) {
        syncRenderedStatus();
        return;
      }
      render(state);
    })
    .catch(() => {});
}

function buildCommandPreview(action, teacherBackend, sessionName) {
  return action.command_template
    .replace("<mock|local-llm>", teacherBackend)
    .replace("\"<session-name>\"", `"${sessionName}"`);
}

function normalizeTeacherBackend(value) {
  if (value === "local_llm") return "local-llm";
  return value || "mock";
}

function ensureSetupSelections(setup) {
  if (!setup) return;
  if (!UI_STATE.selectedRunMode) UI_STATE.selectedRunMode = setup.configured_run_mode;
  if (!UI_STATE.selectedCurriculumSizeHint) UI_STATE.selectedCurriculumSizeHint = setup.curriculum_size_hint;
  if (!UI_STATE.selectedModelPath) UI_STATE.selectedModelPath = setup.model_path;
  if (!UI_STATE.selectedEndpoint) UI_STATE.selectedEndpoint = setup.endpoint;
  if (!UI_STATE.selectedSessionsDir) UI_STATE.selectedSessionsDir = setup.sessions_dir;
  if (!UI_STATE.selectedAggregatedDir) UI_STATE.selectedAggregatedDir = setup.aggregated_dir;
}

function hasSetupChanges(setup) {
  if (!setup) return false;
  return (
    (UI_STATE.selectedRunMode ?? "") !== setup.configured_run_mode
    || (UI_STATE.selectedCurriculumSizeHint ?? "") !== setup.curriculum_size_hint
    || (UI_STATE.selectedModelPath ?? "") !== setup.model_path
    || (UI_STATE.selectedEndpoint ?? "") !== setup.endpoint
    || (UI_STATE.selectedSessionsDir ?? "") !== setup.sessions_dir
    || (UI_STATE.selectedAggregatedDir ?? "") !== setup.aggregated_dir
  );
}

function buildSetupRequest() {
  return {
    run_mode: UI_STATE.selectedRunMode,
    curriculum_size_hint: UI_STATE.selectedCurriculumSizeHint,
    model_path: UI_STATE.selectedModelPath,
    endpoint: UI_STATE.selectedEndpoint,
    sessions_dir: UI_STATE.selectedSessionsDir,
    aggregated_dir: UI_STATE.selectedAggregatedDir,
  };
}

function resetSetupSelections() {
  UI_STATE.selectedRunMode = null;
  UI_STATE.selectedCurriculumSizeHint = null;
  UI_STATE.selectedModelPath = null;
  UI_STATE.selectedEndpoint = null;
  UI_STATE.selectedSessionsDir = null;
  UI_STATE.selectedAggregatedDir = null;
}

function renderError(error) {
  document.getElementById("hero-meta").innerHTML = `<span>GUI state unavailable: ${error.message}</span>`;
}

function initializeSplash(state) {
  const splash = state.splash ?? {};
  const showSplash = splash.show_splash !== false;
  const durationMs = Number.isFinite(splash.duration_ms) ? splash.duration_ms : 3000;
  const assetPath = splash.asset_path || "/assets/fmi-splash-wordmark.png";
  const label = splash.label || "Fractal Media Infrastructure";
  const splashRoot = document.getElementById("startup-splash");
  const splashLogo = document.getElementById("startup-splash-logo");
  const splashLabel = splashRoot?.querySelector(".startup-splash__label");

  if (!splashRoot) {
    revealMainUi();
    UI_STATE.splashDismissed = true;
    return;
  }

  if (splashLogo) {
    splashLogo.src = assetPath;
    splashLogo.alt = label;
  }
  if (splashLabel) {
    splashLabel.textContent = label;
  }

  if (!showSplash) {
    dismissSplash();
    return;
  }

  if (!UI_STATE.splashListenersBound) {
    splashRoot.addEventListener("click", dismissSplash);
    window.addEventListener("keydown", handleSplashKeydown);
    UI_STATE.splashListenersBound = true;
  }

  if (UI_STATE.splashDismissed) return;

  if (UI_STATE.splashTimerHandle) {
    window.clearTimeout(UI_STATE.splashTimerHandle);
  }
  UI_STATE.splashTimerHandle = window.setTimeout(dismissSplash, Math.max(0, durationMs));
}

function handleSplashKeydown(event) {
  if (![" ", "Enter", "Escape", "Spacebar"].includes(event.key)) return;
  event.preventDefault();
  dismissSplash();
}

function dismissSplash() {
  if (UI_STATE.splashDismissed) return;
  UI_STATE.splashDismissed = true;
  if (UI_STATE.splashTimerHandle) {
    window.clearTimeout(UI_STATE.splashTimerHandle);
    UI_STATE.splashTimerHandle = null;
  }
  if (UI_STATE.splashListenersBound) {
    window.removeEventListener("keydown", handleSplashKeydown);
    UI_STATE.splashListenersBound = false;
  }
  revealMainUi();
}

function revealMainUi() {
  document.getElementById("startup-splash")?.classList.add("startup-splash--hidden");
  document.getElementById("app-shell")?.classList.remove("shell--startup-hidden");
}

function initializeTooltips() {
  if (UI_STATE.tooltipBound) return;

  const tooltip = document.getElementById("ui-tooltip");
  if (!tooltip) return;

  let activeTarget = null;

  const showTooltip = (target, event) => {
    const text = target?.getAttribute("data-tooltip");
    if (!text) return;
    activeTarget = target;
    tooltip.textContent = text;
    tooltip.classList.add("is-visible");
    tooltip.setAttribute("aria-hidden", "false");
    positionTooltip(target, event, tooltip);
  };

  const hideTooltip = () => {
    activeTarget = null;
    tooltip.classList.remove("is-visible");
    tooltip.setAttribute("aria-hidden", "true");
  };

  document.addEventListener("mouseover", (event) => {
    const target = event.target.closest("[data-tooltip]");
    if (!target) return;
    showTooltip(target, event);
  });

  document.addEventListener("mouseout", (event) => {
    const source = event.target.closest("[data-tooltip]");
    if (!source) return;
    const next = event.relatedTarget?.closest?.("[data-tooltip]") ?? null;
    if (source === next) return;
    hideTooltip();
  });

  document.addEventListener("mousemove", (event) => {
    if (!activeTarget) return;
    positionTooltip(activeTarget, event, tooltip);
  });

  document.addEventListener("focusin", (event) => {
    const target = event.target.closest("[data-tooltip]");
    if (!target) return;
    showTooltip(target, null);
  });

  document.addEventListener("focusout", (event) => {
    const target = event.target.closest("[data-tooltip]");
    if (!target) return;
    hideTooltip();
  });

  window.addEventListener("scroll", () => {
    if (!activeTarget) return;
    positionTooltip(activeTarget, null, tooltip);
  }, true);

  UI_STATE.tooltipBound = true;
}

function positionTooltip(target, event, tooltip) {
  if (!target || !tooltip) return;

  const margin = 14;
  let left;
  let top;

  if (event?.clientX !== undefined && event?.clientY !== undefined) {
    left = event.clientX + 16;
    top = event.clientY + 18;
  } else {
    const rect = target.getBoundingClientRect();
    left = rect.left + rect.width / 2;
    top = rect.top - 12;
  }

  tooltip.style.left = "0px";
  tooltip.style.top = "0px";

  const { offsetWidth, offsetHeight } = tooltip;
  const maxLeft = window.innerWidth - offsetWidth - margin;
  const maxTop = window.innerHeight - offsetHeight - margin;

  if (!event) {
    left = left - offsetWidth / 2;
    top = top - offsetHeight;
  }

  left = Math.min(Math.max(margin, left), Math.max(margin, maxLeft));
  top = Math.min(Math.max(margin, top), Math.max(margin, maxTop));

  tooltip.style.left = `${left}px`;
  tooltip.style.top = `${top}px`;
}

function bootstrapStartupSplash() {
  const splashRoot = document.getElementById("startup-splash");
  if (!splashRoot || UI_STATE.splashDismissed) {
    revealMainUi();
    return;
  }

  if (!UI_STATE.splashListenersBound) {
    splashRoot.addEventListener("click", dismissSplash);
    window.addEventListener("keydown", handleSplashKeydown);
    UI_STATE.splashListenersBound = true;
  }

  if (UI_STATE.splashTimerHandle) {
    window.clearTimeout(UI_STATE.splashTimerHandle);
  }
  UI_STATE.splashTimerHandle = window.setTimeout(dismissSplash, 3000);
}

function renderLiveJobDetail(job) {
  if (!job) {
    return `
      <p>No active host job.</p>
      <p>Recent job history remains available in the Controls panel.</p>
    `;
  }

  const progress = job.progress;
  const totalExpected = progress?.total_items_expected ?? "?";
  const totalCompleted = progress?.total_items_completed ?? 0;
  const latestItemId = progress?.latest_item_id ?? "n/a";
  const latestItemType = progress?.latest_item_type ?? "n/a";
  const rootDir = progress?.root_dir ?? "n/a";
  const sessionId = progress?.session_id ?? "n/a";
  const runId = progress?.run_id ?? "n/a";

  return `
    <div class="live-job-panel live-job-panel--${job.state}">
      <p>Action: <strong>${job.action}</strong></p>
      <p>State: <strong>${job.state}</strong></p>
      <p>Teacher backend: <strong>${job.teacher_backend ?? "n/a"}</strong></p>
      <p>Session name: <strong>${job.session_name ?? "n/a"}</strong></p>
      <p>Session id: <code>${sessionId}</code></p>
      <p>Run id: <code>${runId}</code></p>
      <p>Phase: <strong>${progress?.phase ?? "queued"}</strong></p>
      <p>Progress: <strong>${totalCompleted} / ${totalExpected}</strong></p>
      <p>Latest item: <code>${latestItemId}</code></p>
      <p>Latest item type: <strong>${latestItemType}</strong></p>
      <p>Latest domain / concept:
        <strong>${escapeHtml(progress?.latest_domain_id ?? "n/a")} / ${escapeHtml(progress?.latest_concept_id ?? "n/a")}</strong>
      </p>
      <p>Prompt: <strong>${escapeHtml(truncateText(progress?.latest_prompt ?? "n/a", 180))}</strong></p>
      <p>Expected / Janet:
        <strong>${escapeHtml(progress?.latest_expected_answer ?? "n/a")} / ${escapeHtml(progress?.latest_janet_answer ?? "pending")}</strong>
      </p>
      <p>Judgment / feedback:
        <strong>${escapeHtml(progress?.latest_correctness_judgment ?? "n/a")} / ${escapeHtml(progress?.latest_teacher_feedback ?? "n/a")}</strong>
      </p>
      <p>Correct / incorrect: <strong>${progress?.correct_count ?? 0} / ${progress?.incorrect_count ?? 0}</strong></p>
      <p>Refusals / anomalies: <strong>${progress?.refusal_count ?? 0} / ${progress?.anomaly_count ?? 0}</strong></p>
      <p>Probes / memory reads: <strong>${progress?.probe_count ?? 0} / ${progress?.memory_reads ?? 0}</strong></p>
      <p>Root: <code>${rootDir}</code></p>
      <p>Status: <strong>${progress?.message ?? job.result_summary ?? "Host bridge is processing."}</strong></p>
    </div>
  `;
}

function renderAnalysisDetail(state, session) {
  const analysis = session.analysis_snapshot;
  if (!analysis) {
    return `
      <p>No analysis report available for this session yet.</p>
      <p>Run completion is required before post-run findings are generated.</p>
    `;
  }

  return `
    <div class="analysis-panel">
      <div class="analysis-group analysis-group--operator">
        <h3>Operator Guidance</h3>
        ${renderOperatorGuidance(state, session, analysis)}
      </div>
      <div class="analysis-group">
        <h3>Signal Counts</h3>
        <p>Confirmed / boundary / emergent:
          <strong>${analysis.confirmed_count} / ${analysis.boundary_count} / ${analysis.emergent_count}</strong>
        </p>
        <p>Unknown structure candidates: <strong>${analysis.unknown_count}</strong></p>
        <p>Repeated anomaly clusters: <strong>${analysis.repeated_anomaly_cluster_count}</strong></p>
        <p>Category mismatch clusters: <strong>${analysis.category_mismatch_cluster_count}</strong></p>
      </div>
      <div class="analysis-group">
        <h3>Confirmed Signals</h3>
        ${renderStringList(analysis.confirmed_summaries, "No confirmed signal summaries surfaced in this run.")}
      </div>
      <div class="analysis-group">
        <h3>Boundary and Emergent</h3>
        <p><strong>Boundary</strong></p>
        ${renderStringList(analysis.boundary_summaries, "No boundary signal summaries surfaced in this run.")}
        <p><strong>Emergent</strong></p>
        ${renderStringList(analysis.emergent_summaries, "No emergent candidate summaries surfaced in this run.")}
      </div>
      <div class="analysis-group">
        <h3>Caution Notes</h3>
        ${renderStringList(analysis.caution_notes, "No caution notes emitted.")}
      </div>
      <div class="analysis-group">
        <h3>Recommended Next Probes</h3>
        ${renderStringList(analysis.recommended_next_probes, "No next probes recommended yet.")}
      </div>
    </div>
  `;
}

function renderOperatorGuidance(state, session, analysis) {
  const actions = [];
  const refusalCount = numericStat(session.refusal_stats, "refusal_count", 0);
  const anomalyCount = numericStat(session.anomaly_stats, "anomaly_flag_count", 0);
  const memoryReads = numericStat(session.memory_stats, "memory_reads", 0);
  const endpointReady = state.setup_snapshot?.endpoint_ready;
  const teacherBackend = session.teacher_backend_id;

  if (teacherBackend === "local_llm" && endpointReady === false) {
    actions.push("Local teacher backend is selected but the runtime endpoint is not ready. Bring `http://127.0.0.1:8080/v1` online before trusting follow-up teacher-guided runs.");
  }
  if (analysis.recommended_next_probes?.length) {
    actions.push(`Next probe focus: ${analysis.recommended_next_probes[0]}`);
  }
  if (refusalCount > 0) {
    actions.push(`Review refusal-heavy items in Live Telemetry. This run recorded ${refusalCount} refusal events, which likely means deterministic skill coverage or approval scope is still too narrow.`);
  }
  if (anomalyCount > 0) {
    actions.push(`Inspect anomaly-linked items before broad conclusions. ${anomalyCount} anomaly flags were preserved in this run and should be treated as first-class evidence.`);
  }
  if (memoryReads === 0) {
    actions.push("Memory reads stayed at zero. If that is unexpected for the task mix, inspect whether prompts are bypassing the explicit memory pathway.");
  }
  if (
    analysis.confirmed_count === 0 &&
    analysis.boundary_count === 0 &&
    analysis.emergent_count === 0
  ) {
    actions.push("No strong post-run signal has stabilized yet. Keep claims provisional and prioritize additional probe pressure over summarizing capability.");
  }
  if (!actions.length) {
    actions.push("No immediate operator interventions were inferred from this session. Continue with targeted probes or compare against another recent run.");
  }

  return renderStringList(actions, "No operator guidance generated.");
}

function renderExportDetail(session) {
  const artifacts = session.artifacts ?? [];
  const bridgeReady = Boolean(window.__JANET_BRIDGE__?.runGuiAction);
  return `
    <div class="export-panel">
      <div class="export-group">
        <h3>Session Path</h3>
        <p>Root folder: <strong>${escapeHtml(truncateText(session.root_dir, 72))}</strong></p>
        <p>Run id: <code>${escapeHtml(session.run_id)}</code></p>
        <p>Skill profile: <strong>${escapeHtml(formatSkillRunProfile(session.skill_run_snapshot))}</strong></p>
        <div class="button-row">
          <button class="action-button" id="open-session-folder" data-run-id="${escapeHtml(session.run_id)}" data-tooltip="Open the current session folder in the host file explorer."${bridgeReady ? "" : " disabled"}>Open Session Folder</button>
          <button class="action-button" id="export-session-bundle" data-run-id="${escapeHtml(session.run_id)}" data-tooltip="Save a workspace-owned JSON bundle containing the current session summary, reports, and event streams."${bridgeReady ? "" : " disabled"}>Save JSON Bundle</button>
        </div>
        ${bridgeReady ? "" : '<p class="control-note">Session folder opening and bundle export require the live Rust bridge.</p>'}
        <details class="panel-details" data-persist-key="export-session-path">
          <summary data-tooltip="Expand to view the full session path and the exact approved skill set used for this run.">Path details</summary>
          <div class="panel-details__body">
            <p>Session root: <code>${escapeHtml(session.root_dir)}</code></p>
            <p>Approved skill set: <strong>${escapeHtml(formatSkillRunApprovedList(session.skill_run_snapshot))}</strong></p>
          </div>
        </details>
      </div>
      <div class="export-group">
        <h3>Artifacts</h3>
        ${artifacts.length
          ? `<div class="artifact-list">
              ${artifacts
                .map(
                  (artifact) => `
                    <div>
                      <a class="artifact-link" href="${escapeHtml(artifact.download_path)}" target="_blank" rel="noreferrer">${escapeHtml(artifact.label)}</a>
                      <div class="artifact-meta">${escapeHtml(artifact.relative_path)} | ${escapeHtml(artifact.content_type)}</div>
                      <details class="panel-details" data-persist-key="artifact-${escapeHtml(artifact.relative_path)}">
                        <summary data-tooltip="Expand to show the full absolute filesystem path for this artifact.">Full path</summary>
                        <div class="panel-details__body">
                          <div class="artifact-meta">${escapeHtml(artifact.absolute_path)}</div>
                        </div>
                      </details>
                    </div>
                  `,
                )
                .join("")}
            </div>`
          : "<p>No export artifacts detected for this session.</p>"}
      </div>
    </div>
  `;
}

function bindExportDetail(session) {
  const openFolderButton = document.getElementById("open-session-folder");
  const exportBundleButton = document.getElementById("export-session-bundle");

  openFolderButton?.addEventListener("click", async () => {
    try {
      const response = await fetch("/api/open-session-folder", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ run_id: session.run_id }),
      });
      if (!response.ok) {
        const message = await response.text();
        throw new Error(message || `HTTP ${response.status}`);
      }
      const opened = await response.json();
      setStatus(`Opened session folder at ${opened.absolute_path}.`, "success");
      syncRenderedStatus();
    } catch (error) {
      setStatus(`Open session folder failed: ${error.message}`, "error");
      syncRenderedStatus();
    }
  });

  exportBundleButton?.addEventListener("click", async () => {
    try {
      const response = await fetch("/api/session-export", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ run_id: session.run_id }),
      });
      if (!response.ok) {
        const message = await response.text();
        throw new Error(message || `HTTP ${response.status}`);
      }
      const saved = await response.json();
      setStatus(`Saved session bundle to ${saved.relative_path}.`, "success");
      syncRenderedStatus();
      window.open(saved.download_path, "_blank", "noopener,noreferrer");
    } catch (error) {
      setStatus(`Session bundle export failed: ${error.message}`, "error");
      syncRenderedStatus();
    }
  });
}

function renderAboutDetail(state) {
  const expanded = UI_STATE.aboutExpanded;
  return `
    <div class="about-panel">
      <div class="about-group about-group--summary">
        <div class="about-summary-row">
          <p>Steward / license: <strong>Fractal Media Infrastructure / AGPL-3.0-or-later</strong></p>
          <button class="action-button about-toggle" id="about-toggle" type="button" aria-expanded="${expanded ? "true" : "false"}" data-tooltip="Expand or collapse the publisher, source, and license details for this project.">
            ${expanded ? "Hide Details" : "Show Details"}
          </button>
        </div>
        <p>Public GitHub: <strong>Instance001</strong></p>
      </div>
      <div class="about-group${expanded ? "" : " is-hidden"}" id="about-expanded-panel">
        <h3>Project</h3>
        <p>Name / version: <strong>${escapeHtml(state.app_name)} / ${escapeHtml(state.version)}</strong></p>
        <p>Shell role: <strong>Local research console for the Janet School rig</strong></p>
        <p>Focus: <strong>Open-source AI tooling, cognitive scaffolding experiments, and local-first research systems</strong></p>
      </div>
      <div class="about-group${expanded ? "" : " is-hidden"}">
        <h3>Stewardship</h3>
        <p>Publisher/steward: <strong>Fractal Media Infrastructure</strong></p>
        <p>Public GitHub: <strong>Instance001</strong></p>
        <p>Repository: <strong><a href="https://github.com/Instance001/janet-school" target="_blank" rel="noreferrer">github.com/Instance001/janet-school</a></strong></p>
        <p>Outreach note: <strong>Media, demos, and outreach may appear through separate channels over time.</strong></p>
      </div>
      <div class="about-group${expanded ? "" : " is-hidden"}">
        <h3>License</h3>
        <p>SPDX: <strong>AGPL-3.0-or-later</strong></p>
        <p>Warranty: <strong>No warranty is provided with this program.</strong></p>
        <p>Local source/license paths:
          <strong><code>LICENSE</code>, <code>README.md</code>, <code>docs/ABOUT.md</code></strong>
        </p>
      </div>
    </div>
  `;
}

function bindAboutDetail() {
  const toggle = document.getElementById("about-toggle");
  if (!toggle) return;

  toggle.addEventListener("click", () => {
    UI_STATE.aboutExpanded = !UI_STATE.aboutExpanded;
    const detail = document.getElementById("about-detail");
    if (!detail) return;
    detail.innerHTML = renderAboutDetail(window.__JANET_SCHOOL__ ?? { app_name: "Janet School", version: "n/a" });
    bindAboutDetail();
  });
}

function renderCurriculumDetail(session) {
  const preview = session.curriculum_preview;
  if (!preview) {
    return `
      <p>Items: <strong>${session.curriculum_stats.item_count ?? 0}</strong></p>
      <p>Domains: <strong>${session.curriculum_stats.domain_count ?? 0}</strong></p>
      <p>Concepts: <strong>${session.curriculum_stats.concept_count ?? 0}</strong></p>
      <p>Probes: <strong>${session.curriculum_stats.probe_count ?? 0}</strong></p>
    `;
  }

  const selectedDomainId = resolveSelectedCurriculumDomainId(preview);
  const domainSummaries = preview.domain_summaries ?? [];
  const sampleItems = preview.sample_items ?? [];
  const domainItems = sampleItems.filter((item) => item.domain_id === selectedDomainId);
  const selectedItemId = resolveSelectedCurriculumItemId(domainItems);
  const selectedDomain = domainSummaries.find((domain) => domain.domain_id === selectedDomainId) ?? domainSummaries[0];
  const selectedItem = domainItems.find((item) => item.item_id === selectedItemId) ?? domainItems[0] ?? sampleItems[0] ?? null;

  return `
    <div class="curriculum-panel">
      <div class="curriculum-group">
        <h3>Coverage</h3>
        <p>Items: <strong>${session.curriculum_stats.item_count ?? 0}</strong></p>
        <p>Domains: <strong>${session.curriculum_stats.domain_count ?? 0}</strong></p>
        <p>Concepts: <strong>${session.curriculum_stats.concept_count ?? 0}</strong></p>
        <p>Probes: <strong>${session.curriculum_stats.probe_count ?? 0}</strong></p>
        <p>Item mix: <strong>${formatCountMetrics(preview.item_mix)}</strong></p>
      </div>
      <div class="curriculum-group">
        <h3>Generation Notes</h3>
        ${renderStringList(preview.generation_notes, "No generation notes recorded.")}
      </div>
      <div class="curriculum-group">
        <h3>Warnings</h3>
        ${renderStringList(preview.warnings, "No curriculum warnings surfaced.")}
      </div>
      <div class="curriculum-group">
        <h3>Curriculum Inspector</h3>
        <div class="curriculum-inspector">
          <label class="curriculum-control">
            <span>Domain</span>
            <select id="curriculum-domain-select" data-tooltip="Choose which curriculum domain to inspect in the sample-item viewer below.">
              ${domainSummaries
                .map(
                  (domain) => `
                    <option value="${escapeHtml(domain.domain_id)}"${domain.domain_id === selectedDomainId ? " selected" : ""}>
                      ${escapeHtml(domain.name)}
                    </option>
                  `,
                )
                .join("")}
            </select>
          </label>
          <label class="curriculum-control">
            <span>Sample item</span>
            <select id="curriculum-item-select" data-tooltip="Choose a sample item from the selected domain to inspect its prompt, expected answer, and expected skills.">
              ${domainItems.length
                ? domainItems
                    .map(
                      (item) => `
                        <option value="${escapeHtml(item.item_id)}"${item.item_id === selectedItemId ? " selected" : ""}>
                          ${escapeHtml(item.item_id)}
                        </option>
                      `,
                    )
                    .join("")
                : '<option value="">No sample items in this domain</option>'}
            </select>
          </label>
        </div>
        <div class="curriculum-inspector-grid">
          <article class="curriculum-entry curriculum-entry--focus">
            <p><strong>${escapeHtml(selectedDomain?.name ?? "n/a")}</strong> <span>${escapeHtml(selectedDomain?.domain_id ?? "n/a")}</span></p>
            <p>Concepts / items / probes:
              <strong>${selectedDomain?.concept_count ?? 0} / ${selectedDomain?.item_count ?? 0} / ${selectedDomain?.probe_count ?? 0}</strong>
            </p>
            <p>Concept list: <strong>${escapeHtml(formatListInline(selectedDomain?.concepts ?? []))}</strong></p>
          </article>
          <article class="curriculum-entry curriculum-entry--focus">
            ${selectedItem
              ? `
                <p><strong>${escapeHtml(selectedItem.item_id)}</strong></p>
                <p>Domain / concept:
                  <strong>${escapeHtml(selectedItem.domain_id)} / ${escapeHtml(selectedItem.concept_id)}</strong>
                </p>
                <p>Type / novelty / probe:
                  <strong>${escapeHtml(selectedItem.item_type)} / ${escapeHtml(selectedItem.novelty_class)} / ${escapeHtml(selectedItem.probe_role)}</strong>
                </p>
                <p>Boundary kind: <strong>${escapeHtml(selectedItem.boundary_kind)}</strong></p>
                <p>Prompt: <strong>${escapeHtml(selectedItem.prompt)}</strong></p>
                <p>Expected answer: <strong>${escapeHtml(selectedItem.expected_answer ?? "n/a")}</strong></p>
                <p>Expected skills: <strong>${escapeHtml(formatListInline(selectedItem.expected_skills))}</strong></p>
              `
              : `
                <p>No sample item is available for the current domain selection.</p>
              `}
          </article>
        </div>
      </div>
      <div class="curriculum-group">
        <h3>Domain Overview</h3>
        <div class="curriculum-list">
          ${domainSummaries
            .map(
              (domain) => `
                <article class="curriculum-entry">
                  <p><strong>${escapeHtml(domain.name)}</strong> <span>${escapeHtml(domain.domain_id)}</span></p>
                  <p>Concepts / items / probes:
                    <strong>${domain.concept_count} / ${domain.item_count} / ${domain.probe_count}</strong>
                  </p>
                  <p>Concept list: <strong>${escapeHtml(formatListInline(domain.concepts))}</strong></p>
                </article>
              `,
            )
            .join("")}
        </div>
      </div>
    </div>
  `;
}

function bindCurriculumInspector(session) {
  const preview = session.curriculum_preview;
  if (!preview) return;

  const domainSelect = document.getElementById("curriculum-domain-select");
  const itemSelect = document.getElementById("curriculum-item-select");
  if (!domainSelect || !itemSelect) return;

  domainSelect.addEventListener("change", () => {
    UI_STATE.selectedCurriculumDomainId = domainSelect.value || null;
    UI_STATE.selectedCurriculumItemId = null;
    document.getElementById("curriculum-detail").innerHTML = renderCurriculumDetail(session);
    bindCurriculumInspector(session);
  });

  itemSelect.addEventListener("change", () => {
    UI_STATE.selectedCurriculumItemId = itemSelect.value || null;
    document.getElementById("curriculum-detail").innerHTML = renderCurriculumDetail(session);
    bindCurriculumInspector(session);
  });
}

function resolveSelectedCurriculumDomainId(preview) {
  const domains = preview.domain_summaries ?? [];
  if (!domains.length) {
    UI_STATE.selectedCurriculumDomainId = null;
    return null;
  }

  const current = UI_STATE.selectedCurriculumDomainId;
  const exists = domains.some((domain) => domain.domain_id === current);
  const resolved = exists ? current : domains[0].domain_id;
  UI_STATE.selectedCurriculumDomainId = resolved;
  return resolved;
}

function resolveSelectedCurriculumItemId(items) {
  if (!items.length) {
    UI_STATE.selectedCurriculumItemId = null;
    return null;
  }

  const current = UI_STATE.selectedCurriculumItemId;
  const exists = items.some((item) => item.item_id === current);
  const resolved = exists ? current : items[0].item_id;
  UI_STATE.selectedCurriculumItemId = resolved;
  return resolved;
}

function renderRunDetail(session, liveJob) {
  const progress = liveJob?.progress ?? null;
  const liveState = liveJob?.state ?? "idle";
  const runStateClass = ["queued", "running", "cancelling", "pausing", "paused", "completed", "stopped", "failed"].includes(liveState)
    ? liveState
    : "idle";

  return `
    <div class="run-panel">
      <div class="run-group run-group--${runStateClass}">
        <h3>Active Host Execution</h3>
        ${liveJob
          ? `
            <p>Action / state:
              <strong>${escapeHtml(liveJob.action)} / ${escapeHtml(liveJob.state)}</strong>
            </p>
            <p>Session name: <strong>${escapeHtml(liveJob.session_name ?? "n/a")}</strong></p>
            <p>Teacher backend: <strong>${escapeHtml(liveJob.teacher_backend ?? "n/a")}</strong></p>
            <p>Started: <strong>${escapeHtml(liveJob.started_at ?? liveJob.created_at ?? "n/a")}</strong></p>
            <p>Phase / progress:
              <strong>${escapeHtml(progress?.phase ?? "queued")} / ${progress?.total_items_completed ?? 0} / ${progress?.total_items_expected ?? "?"}</strong>
            </p>
            <p>Current item:
              <strong>${escapeHtml(progress?.latest_item_id ?? "n/a")}</strong>
            </p>
            <p>Current item type:
              <strong>${escapeHtml(progress?.latest_item_type ?? "n/a")}</strong>
            </p>
            <p>Current domain / concept:
              <strong>${escapeHtml(progress?.latest_domain_id ?? "n/a")} / ${escapeHtml(progress?.latest_concept_id ?? "n/a")}</strong>
            </p>
            <p>Prompt:
              <strong>${escapeHtml(truncateText(progress?.latest_prompt ?? "n/a", 180))}</strong>
            </p>
            <p>Expected / Janet:
              <strong>${escapeHtml(progress?.latest_expected_answer ?? "n/a")} / ${escapeHtml(progress?.latest_janet_answer ?? "pending")}</strong>
            </p>
            <p>Current judgment:
              <strong>${escapeHtml(progress?.latest_correctness_judgment ?? "n/a")}</strong>
            </p>
            <p>Teacher feedback:
              <strong>${escapeHtml(progress?.latest_teacher_feedback ?? "n/a")}</strong>
            </p>
            <p>Correct / incorrect / refusals:
              <strong>${progress?.correct_count ?? 0} / ${progress?.incorrect_count ?? 0} / ${progress?.refusal_count ?? 0}</strong>
            </p>
            <p>Anomalies / probes / memory reads:
              <strong>${progress?.anomaly_count ?? 0} / ${progress?.probe_count ?? 0} / ${progress?.memory_reads ?? 0}</strong>
            </p>
            <p>Status: <strong>${escapeHtml(progress?.message ?? liveJob.result_summary ?? "Host bridge is processing.")}</strong></p>
          `
          : `
            <p>No active bridge job is running right now.</p>
            <p>The latest completed session metrics remain available below.</p>
          `}
      </div>
      <div class="run-group run-group--completed">
        <h3>Latest Completed Session</h3>
        <p>Run id: <code>${escapeHtml(session.run_id)}</code></p>
        <p>Completed: <strong>${escapeHtml(session.completed_at ?? "in progress")}</strong></p>
        <p>Teacher backend: <strong>${escapeHtml(session.teacher_backend_id)}</strong></p>
        <p>Skill profile: <strong>${escapeHtml(formatSkillRunProfile(session.skill_run_snapshot))}</strong></p>
        <p>Blocked skill count: <strong>${escapeHtml(formatSkillRunBlockedCount(session.skill_run_snapshot))}</strong></p>
        <p>Total items / probes:
          <strong>${session.interaction_stats.total_items ?? 0} / ${session.interaction_stats.probe_count ?? 0}</strong>
        </p>
        <p>Correct / incorrect:
          <strong>${session.interaction_stats.correct_count ?? 0} / ${session.interaction_stats.incorrect_count ?? 0}</strong>
        </p>
        <p>Refusals / anomaly flags:
          <strong>${session.refusal_stats.refusal_count ?? 0} / ${session.anomaly_stats.anomaly_flag_count ?? 0}</strong>
        </p>
        <p>Memory reads: <strong>${session.memory_stats.memory_reads ?? 0}</strong></p>
        <details class="panel-details" data-persist-key="run-latest-details">
          <summary data-tooltip="Expand to view the full approved skill set and session note for the latest completed run.">More details</summary>
          <div class="panel-details__body">
            <p>Approved skill set: <strong>${escapeHtml(formatSkillRunApprovedList(session.skill_run_snapshot))}</strong></p>
            <p>Session note: <strong>${escapeHtml(session.notes?.[0] ?? "n/a")}</strong></p>
          </div>
        </details>
      </div>
    </div>
  `;
}

function renderSetupDetail(state, latestSession) {
  const setup = state.setup_snapshot;
  const bridgeReady = Boolean(window.__JANET_BRIDGE__?.runGuiAction);
  if (!setup) {
    return `
      <p>Run mode: <strong>${latestSession.run_mode}</strong></p>
      <p>Teacher backend: <strong>${latestSession.teacher_backend_id}</strong></p>
      <p>Session: <code>${latestSession.session_id}</code></p>
      <p>Root: <code>${latestSession.root_dir}</code></p>
    `;
  }

  ensureSetupSelections(setup);
  const dirtySetup = hasSetupChanges(setup);

  return `
    <div class="export-panel">
      <div class="export-group">
        <h3>Configuration</h3>
        <p>Environment: <strong>${escapeHtml(setup.environment)}</strong></p>
        <label class="curriculum-control">
          <span>Run mode</span>
          <select id="setup-run-mode" data-tooltip="Choose the default run mode used by GUI-triggered sessions. Analysis-only is for report refresh workflows without a normal execution pass." ${UI_STATE.actionRunning ? "disabled" : ""}>
            <option value="smoke"${UI_STATE.selectedRunMode === "smoke" ? " selected" : ""}>smoke</option>
            <option value="full"${UI_STATE.selectedRunMode === "full" ? " selected" : ""}>full</option>
            <option value="analysis_only"${UI_STATE.selectedRunMode === "analysis_only" ? " selected" : ""}>analysis_only</option>
          </select>
        </label>
        <label class="curriculum-control">
          <span>Curriculum size</span>
          <select id="setup-curriculum-size" data-tooltip="Choose how large generated curricula should be by default. Full should become the normal research profile once the larger curriculum target is complete." ${UI_STATE.actionRunning ? "disabled" : ""}>
            <option value="tiny_fixture"${UI_STATE.selectedCurriculumSizeHint === "tiny_fixture" ? " selected" : ""}>tiny_fixture</option>
            <option value="smoke"${UI_STATE.selectedCurriculumSizeHint === "smoke" ? " selected" : ""}>smoke</option>
            <option value="full"${UI_STATE.selectedCurriculumSizeHint === "full" ? " selected" : ""}>full</option>
          </select>
        </label>
        <details class="panel-details" data-persist-key="setup-directory-paths">
          <summary data-tooltip="Expand to view the full configured session and aggregate directory paths.">Directory paths</summary>
          <div class="panel-details__body">
            <label class="curriculum-control">
              <span>Session output dir</span>
              <input id="setup-sessions-dir" type="text" value="${escapeHtml(UI_STATE.selectedSessionsDir)}" data-tooltip="Directory where per-run session folders are written." ${UI_STATE.actionRunning ? "disabled" : ""} />
            </label>
            <label class="curriculum-control">
              <span>Aggregate output dir</span>
              <input id="setup-aggregated-dir" type="text" value="${escapeHtml(UI_STATE.selectedAggregatedDir)}" data-tooltip="Directory used for aggregated outputs and future cross-session material." ${UI_STATE.actionRunning ? "disabled" : ""} />
            </label>
          </div>
        </details>
        <div class="button-row">
          <button class="action-button" id="setup-save-button" data-tooltip="Persist the current setup values into the project config files so future GUI and CLI runs inherit them."${UI_STATE.actionRunning || !dirtySetup || !bridgeReady ? " disabled" : ""}>Save Setup</button>
        </div>
        <p class="control-note" id="setup-dirty-note">${dirtySetup ? "Unsaved setup changes are staged locally and will be applied to new GUI-triggered runs." : "Setup matches the saved project config."}</p>
        ${bridgeReady ? "" : '<p class="control-note">Setup persistence requires the live Rust bridge. Static shell mode is read-only.</p>'}
      </div>
      <div class="export-group">
        <h3>Runtime Readiness</h3>
        <p>Runtime enabled: <strong>${setup.runtime_enabled ? "yes" : "no"}</strong></p>
        <p>Endpoint ready: <strong>${setup.endpoint_ready ? "yes" : "no"}</strong></p>
        <p>Runtime path: <strong>${setup.runtime_path_exists ? "present" : "missing"}</strong></p>
        <p>Server binary: <strong>${setup.server_binary_exists ? "present" : "missing"}</strong></p>
        <p>Model file: <strong>${setup.model_path_exists ? "present" : "missing"}</strong></p>
        <label class="curriculum-control">
          <span>Local endpoint</span>
          <input id="setup-endpoint" type="text" value="${escapeHtml(UI_STATE.selectedEndpoint)}" data-tooltip="OpenAI-compatible local teacher endpoint used when the local LLM backend is selected." ${UI_STATE.actionRunning ? "disabled" : ""} />
        </label>
      </div>
      <div class="export-group">
        <h3>Local Model</h3>
        <label class="curriculum-control">
          <span>Model path</span>
          <input id="setup-model-path" type="text" value="${escapeHtml(UI_STATE.selectedModelPath)}" data-tooltip="Filesystem path to the local teacher model file used by runtime launch and readiness checks." ${UI_STATE.actionRunning ? "disabled" : ""} />
        </label>
        <p>Context / GPU layers: <strong>${setup.context_size} / ${setup.gpu_layers}</strong></p>
        <p>Latest session: <code>${escapeHtml(latestSession.session_id)}</code></p>
        <details class="panel-details" data-persist-key="setup-model-root-details">
          <summary data-tooltip="Expand to view the full model path and latest session root path.">Model and root details</summary>
          <div class="panel-details__body">
            <p>Configured model path: <code>${escapeHtml(setup.model_path)}</code></p>
            <p>Latest root: <code>${escapeHtml(latestSession.root_dir)}</code></p>
          </div>
        </details>
      </div>
      <div class="export-group">
        <h3>Warnings</h3>
        ${renderStringList(setup.warnings, "No setup warnings surfaced.")}
      </div>
    </div>
  `;
}

function bindSetupDetail(state) {
  const setup = state.setup_snapshot;
  if (!setup) return;

  const runModeSelect = document.getElementById("setup-run-mode");
  const sizeSelect = document.getElementById("setup-curriculum-size");
  const modelInput = document.getElementById("setup-model-path");
  const endpointInput = document.getElementById("setup-endpoint");
  const sessionsInput = document.getElementById("setup-sessions-dir");
  const aggregatedInput = document.getElementById("setup-aggregated-dir");
  const saveButton = document.getElementById("setup-save-button");

  const syncSetupDirtyIndicators = () => {
    const dirty = hasSetupChanges(setup);
    if (saveButton) {
      saveButton.disabled = UI_STATE.actionRunning || !dirty || !window.__JANET_BRIDGE__?.runGuiAction;
    }
    const dirtyNote = document.getElementById("setup-dirty-note");
    if (dirtyNote) {
      dirtyNote.textContent = dirty
        ? "Unsaved setup changes are staged locally and will be applied to new GUI-triggered runs."
        : "Setup matches the saved project config.";
    }
  };

  runModeSelect?.addEventListener("change", () => {
    UI_STATE.selectedRunMode = runModeSelect.value;
    syncSetupDirtyIndicators();
  });
  sizeSelect?.addEventListener("change", () => {
    UI_STATE.selectedCurriculumSizeHint = sizeSelect.value;
    syncSetupDirtyIndicators();
  });
  modelInput?.addEventListener("input", () => {
    UI_STATE.selectedModelPath = modelInput.value;
    syncSetupDirtyIndicators();
  });
  endpointInput?.addEventListener("input", () => {
    UI_STATE.selectedEndpoint = endpointInput.value;
    syncSetupDirtyIndicators();
  });
  sessionsInput?.addEventListener("input", () => {
    UI_STATE.selectedSessionsDir = sessionsInput.value;
    syncSetupDirtyIndicators();
  });
  aggregatedInput?.addEventListener("input", () => {
    UI_STATE.selectedAggregatedDir = aggregatedInput.value;
    syncSetupDirtyIndicators();
  });

  saveButton?.addEventListener("click", async () => {
    try {
      UI_STATE.actionRunning = true;
      setStatus("Saving setup into project config files.", "running");
      syncRenderedStatus();
      const accepted = await window.__JANET_BRIDGE__.runGuiAction({
        action: "save_setup",
        teacher_backend: UI_STATE.selectedTeacherBackend ?? state.configured_teacher_backend ?? state.teacher_backend,
        ...buildSetupRequest(),
      });
      UI_STATE.actionRunning = false;
      UI_STATE.activeJobId = accepted.job.job_id;
      UI_STATE.activeJobState = accepted.job.state;
      resetSetupSelections();
      setStatus(`Setup saved through host job ${accepted.job.job_id.slice(0, 8)}.`, "success");
      await loadGuiState({ silent: true });
    } catch (error) {
      UI_STATE.actionRunning = false;
      setStatus(`Setup save failed: ${error.message}`, "error");
      syncRenderedStatus();
    }
  });

  syncSetupDirtyIndicators();
}

function renderRecentSessions(sessions) {
  if (!sessions?.length) {
    return "<p>No recent sessions.</p>";
  }

  return `
    <div class="recent-list">
      ${sessions.map((session, index) => renderRecentSessionCard(session, index === 0)).join("")}
    </div>
  `;
}

function renderCompareSessions(sessions) {
  if (!sessions?.length) {
    return "<p>No sessions available to compare yet.</p>";
  }

  const [primary, secondary] = resolveCompareSessions(sessions);
  if (!primary || !secondary) {
    return "<p>At least two sessions are needed for comparison.</p>";
  }

  const compareFilterOptions = buildCompareFilterOptions(primary, secondary);

  return `
    <div class="compare-panel">
      <div class="compare-group">
        <h3>Compare Selection</h3>
        <div class="compare-controls">
          <label class="curriculum-control">
            <span>Primary run</span>
            <select id="compare-primary-select" data-tooltip="Choose the primary run for comparison. Its outcomes appear on the primary side of the compare view.">
              ${sessions
                .map(
                  (session) => `
                    <option value="${escapeHtml(session.run_id)}"${session.run_id === primary.run_id ? " selected" : ""}>
                      ${escapeHtml(session.run_id.slice(0, 8))} | ${escapeHtml(formatSkillRunProfile(session.skill_run_snapshot))}
                    </option>
                  `,
                )
                .join("")}
            </select>
          </label>
          <label class="curriculum-control">
            <span>Secondary run</span>
            <select id="compare-secondary-select" data-tooltip="Choose the secondary run to compare against the primary run.">
              ${sessions
                .map(
                  (session) => `
                    <option value="${escapeHtml(session.run_id)}"${session.run_id === secondary.run_id ? " selected" : ""}>
                      ${escapeHtml(session.run_id.slice(0, 8))} | ${escapeHtml(formatSkillRunProfile(session.skill_run_snapshot))}
                    </option>
                  `,
                )
                .join("")}
            </select>
          </label>
        </div>
      </div>
      <div class="compare-group">
        <h3>Run Profiles</h3>
        <div class="compare-grid">
          ${renderCompareRunCard("Primary", primary)}
          ${renderCompareRunCard("Secondary", secondary)}
        </div>
      </div>
      <div class="compare-group">
        <h3>High-Signal Deltas</h3>
        ${renderCompareDeltas(primary, secondary)}
      </div>
      <div class="compare-group">
        <h3>Export Compare View</h3>
        <p>Save the current run pairing, filter state, and compare findings into <code>compare_exports/</code> inside this workspace.</p>
        <div class="compare-export-controls">
          <label class="curriculum-control">
            <span>Export scope</span>
            <select id="compare-export-scope" data-tooltip="Choose whether export should include only visible compare rows or the full filtered result set.">
              <option value="visible"${UI_STATE.compareExportScope === "visible" ? " selected" : ""}>Visible items only</option>
              <option value="all_filtered"${UI_STATE.compareExportScope === "all_filtered" ? " selected" : ""}>All filtered items</option>
            </select>
          </label>
        </div>
        <div class="button-row compare-export-actions">
          <button class="action-button" id="compare-export-markdown" data-tooltip="Save the current compare view as a Markdown report into compare_exports in this workspace.">Save Markdown</button>
          <button class="action-button" id="compare-export-json" data-tooltip="Save the current compare view as structured JSON into compare_exports in this workspace.">Save JSON</button>
        </div>
        ${renderLastCompareExport()}
      </div>
      <div class="compare-group">
        <h3>Overlapping Item Outcomes</h3>
        <div class="button-row compare-presets">
          <button class="action-button" data-compare-preset="changed_refusals" data-tooltip="Show compare rows where refusal behavior changed between runs.">Only changed refusals</button>
          <button class="action-button" data-compare-preset="anomaly_shifts" data-tooltip="Show compare rows where anomaly flags changed between runs.">Only anomaly shifts</button>
          <button class="action-button" data-compare-preset="skill_shifts" data-tooltip="Show compare rows where the executed skill path changed between runs.">Only skill-execution changes</button>
          <button class="action-button" data-compare-preset="reset" data-tooltip="Clear the compare presets and restore the default filter state.">Reset filters</button>
        </div>
        <div class="compare-filters">
          <label class="compare-toggle" data-tooltip="Limit the overlap list to items whose meaningful outcomes changed between the two runs.">
            <input type="checkbox" id="compare-changed-only"${UI_STATE.compareShowChangedOnly ? " checked" : ""} />
            <span>Changed only</span>
          </label>
          <label class="compare-toggle" data-tooltip="Limit the overlap list to items involving refusal behavior.">
            <input type="checkbox" id="compare-refusals-only"${UI_STATE.compareShowRefusalsOnly ? " checked" : ""} />
            <span>Refusals only</span>
          </label>
          <label class="compare-toggle" data-tooltip="Limit the overlap list to items carrying anomaly flags.">
            <input type="checkbox" id="compare-anomalies-only"${UI_STATE.compareShowAnomaliesOnly ? " checked" : ""} />
            <span>Anomalies only</span>
          </label>
          <label class="curriculum-control">
            <span>Domain</span>
            <select id="compare-domain-filter" data-tooltip="Filter the overlap list down to one curriculum domain.">
              <option value="all">All domains</option>
              ${compareFilterOptions.domains
                .map(
                  (domainId) => `
                    <option value="${escapeHtml(domainId)}"${domainId === UI_STATE.compareDomainFilter ? " selected" : ""}>
                      ${escapeHtml(domainId)}
                    </option>
                  `,
                )
                .join("")}
            </select>
          </label>
          <label class="curriculum-control">
            <span>Item type</span>
            <select id="compare-item-type-filter" data-tooltip="Filter the overlap list to a specific item type such as teaching or probe items.">
              <option value="all">All item types</option>
              ${compareFilterOptions.itemTypes
                .map(
                  (itemType) => `
                    <option value="${escapeHtml(itemType)}"${itemType === UI_STATE.compareItemTypeFilter ? " selected" : ""}>
                      ${escapeHtml(itemType)}
                    </option>
                  `,
                )
                .join("")}
            </select>
          </label>
        </div>
        ${renderCompareOverlaps(primary, secondary)}
      </div>
    </div>
  `;
}

function renderCompareRunCard(label, session) {
  return `
    <article class="compare-card">
      <p class="recent-kicker">${escapeHtml(label)}</p>
      <p><strong>${escapeHtml(session.run_id)}</strong></p>
      <p>Completed: <strong>${escapeHtml(session.completed_at ?? "in progress")}</strong></p>
      <p>Teacher backend: <strong>${escapeHtml(session.teacher_backend_id)}</strong></p>
      <p>Skill profile: <strong>${escapeHtml(formatSkillRunProfile(session.skill_run_snapshot))}</strong></p>
      <p>Approved skills: <strong>${escapeHtml(formatSkillRunApprovedList(session.skill_run_snapshot))}</strong></p>
      <p>Accuracy: <strong>${formatAccuracy(session)}</strong></p>
      <p>Items / probes:
        <strong>${numericStat(session.interaction_stats, "total_items", 0)} / ${numericStat(session.interaction_stats, "probe_count", 0)}</strong>
      </p>
      <p>Refusals / anomalies:
        <strong>${numericStat(session.refusal_stats, "refusal_count", 0)} / ${numericStat(session.anomaly_stats, "anomaly_flag_count", 0)}</strong>
      </p>
      <p>Signals:
        <strong>${numericStat(session.analysis_snapshot, "confirmed_count", 0)} confirmed / ${numericStat(session.analysis_snapshot, "boundary_count", 0)} boundary / ${numericStat(session.analysis_snapshot, "emergent_count", 0)} emergent</strong>
      </p>
    </article>
  `;
}

function renderCompareDeltas(primary, secondary) {
  return renderStringList(buildCompareDeltaList(primary, secondary), "No delta summary is available.");
}

function renderCompareOverlaps(primary, secondary) {
  const compareContext = buildCompareContext(primary, secondary);
  const overlapping = compareContext.overlapping;
  const visible = compareContext.visible;

  if (!overlapping.length) {
    return "<p>No overlapping item ids were available between these runs.</p>";
  }

  if (!visible.length) {
    return `
      <p>Overlapping items compared: <strong>${overlapping.length}</strong></p>
      <p>No overlapping items matched the current compare filters.</p>
    `;
  }

  return `
    <p>Overlapping items compared: <strong>${overlapping.length}</strong></p>
    <p>Visible after filters: <strong>${visible.length}</strong></p>
    <div class="compare-overlap-list">
      ${visible
        .map(
          ({ itemId, domainId, primaryItem, secondaryItem, changed, hasRefusal, hasAnomaly, hasSkillShift }) => `
            <article class="compare-card">
              <p><strong>${escapeHtml(itemId)}</strong> <span>${escapeHtml(primaryItem.item_type)}</span></p>
              <p>Domain / change flags:
                <strong>${escapeHtml(domainId)} / ${changed ? "changed" : "same"} / ${hasRefusal ? "refusal" : "no refusal"} / ${hasAnomaly ? "anomaly" : "no anomaly"} / ${hasSkillShift ? "skill shift" : "same skill path"}</strong>
              </p>
              <p>Prompt: <strong>${escapeHtml(truncateText(primaryItem.prompt, 180))}</strong></p>
              <p>Expected answer: <strong>${escapeHtml(primaryItem.expected_answer ?? "n/a")}</strong></p>
              <p>Primary answer / judgment / skill:
                <strong>${escapeHtml(primaryItem.janet_answer ?? "refused")} / ${escapeHtml(primaryItem.correctness_judgment)} / ${escapeHtml(primaryItem.executed_skill ?? "n/a")}</strong>
              </p>
              <p>Secondary answer / judgment / skill:
                <strong>${escapeHtml(secondaryItem.janet_answer ?? "refused")} / ${escapeHtml(secondaryItem.correctness_judgment)} / ${escapeHtml(secondaryItem.executed_skill ?? "n/a")}</strong>
              </p>
              <p>Primary fit / refusal / anomalies:
                <strong>${escapeHtml(primaryItem.structure_fit)} / ${escapeHtml(primaryItem.refusal_reason ?? "n/a")} / ${escapeHtml(formatListInline(primaryItem.anomaly_flags))}</strong>
              </p>
              <p>Secondary fit / refusal / anomalies:
                <strong>${escapeHtml(secondaryItem.structure_fit)} / ${escapeHtml(secondaryItem.refusal_reason ?? "n/a")} / ${escapeHtml(formatListInline(secondaryItem.anomaly_flags))}</strong>
              </p>
            </article>
          `,
        )
        .join("")}
    </div>
  `;
}

function bindCompareSessions(sessions) {
  const primarySelect = document.getElementById("compare-primary-select");
  const secondarySelect = document.getElementById("compare-secondary-select");
  const changedOnly = document.getElementById("compare-changed-only");
  const refusalsOnly = document.getElementById("compare-refusals-only");
  const anomaliesOnly = document.getElementById("compare-anomalies-only");
  const domainFilter = document.getElementById("compare-domain-filter");
  const itemTypeFilter = document.getElementById("compare-item-type-filter");
  const exportScope = document.getElementById("compare-export-scope");
  const presetButtons = document.querySelectorAll("[data-compare-preset]");
  const exportMarkdownButton = document.getElementById("compare-export-markdown");
  const exportJsonButton = document.getElementById("compare-export-json");
  if (!primarySelect || !secondarySelect) return;

  primarySelect.addEventListener("change", () => {
    UI_STATE.comparePrimaryRunId = primarySelect.value || null;
    document.getElementById("compare-sessions").innerHTML = renderCompareSessions(sessions);
    bindCompareSessions(sessions);
  });

  secondarySelect.addEventListener("change", () => {
    UI_STATE.compareSecondaryRunId = secondarySelect.value || null;
    document.getElementById("compare-sessions").innerHTML = renderCompareSessions(sessions);
    bindCompareSessions(sessions);
  });

  changedOnly?.addEventListener("change", () => {
    UI_STATE.compareShowChangedOnly = changedOnly.checked;
    document.getElementById("compare-sessions").innerHTML = renderCompareSessions(sessions);
    bindCompareSessions(sessions);
  });

  refusalsOnly?.addEventListener("change", () => {
    UI_STATE.compareShowRefusalsOnly = refusalsOnly.checked;
    document.getElementById("compare-sessions").innerHTML = renderCompareSessions(sessions);
    bindCompareSessions(sessions);
  });

  anomaliesOnly?.addEventListener("change", () => {
    UI_STATE.compareShowAnomaliesOnly = anomaliesOnly.checked;
    document.getElementById("compare-sessions").innerHTML = renderCompareSessions(sessions);
    bindCompareSessions(sessions);
  });

  domainFilter?.addEventListener("change", () => {
    UI_STATE.compareDomainFilter = domainFilter.value || "all";
    document.getElementById("compare-sessions").innerHTML = renderCompareSessions(sessions);
    bindCompareSessions(sessions);
  });

  itemTypeFilter?.addEventListener("change", () => {
    UI_STATE.compareItemTypeFilter = itemTypeFilter.value || "all";
    document.getElementById("compare-sessions").innerHTML = renderCompareSessions(sessions);
    bindCompareSessions(sessions);
  });

  exportScope?.addEventListener("change", () => {
    UI_STATE.compareExportScope = exportScope.value || "visible";
    document.getElementById("compare-sessions").innerHTML = renderCompareSessions(sessions);
    bindCompareSessions(sessions);
  });

  presetButtons.forEach((button) => {
    button.addEventListener("click", () => {
      applyComparePreset(button.dataset.comparePreset);
      document.getElementById("compare-sessions").innerHTML = renderCompareSessions(sessions);
      bindCompareSessions(sessions);
    });
  });

  exportMarkdownButton?.addEventListener("click", async () => {
    const [primary, secondary] = resolveCompareSessions(sessions);
    if (!primary || !secondary) return;
    await saveCompareExport("markdown", primary, secondary);
  });

  exportJsonButton?.addEventListener("click", async () => {
    const [primary, secondary] = resolveCompareSessions(sessions);
    if (!primary || !secondary) return;
    await saveCompareExport("json", primary, secondary);
  });
}

function resolveCompareSessions(sessions) {
  if (!sessions?.length) return [null, null];
  const primary = sessions.find((session) => session.run_id === UI_STATE.comparePrimaryRunId) ?? sessions[0] ?? null;
  const fallbackSecondary = sessions.find((session) => session.run_id !== primary?.run_id) ?? sessions[0] ?? null;
  const secondary = sessions.find((session) => session.run_id === UI_STATE.compareSecondaryRunId && session.run_id !== primary?.run_id)
    ?? fallbackSecondary;

  UI_STATE.comparePrimaryRunId = primary?.run_id ?? null;
  UI_STATE.compareSecondaryRunId = secondary?.run_id ?? null;
  return [primary, secondary];
}

function renderRecentSessionCard(session, isLatest) {
  const items = numericStat(session.interaction_stats, "total_items", session.curriculum_stats?.item_count ?? 0);
  const probes = numericStat(session.interaction_stats, "probe_count", session.curriculum_stats?.probe_count ?? 0);
  const correct = numericStat(session.interaction_stats, "correct_count", 0);
  const incorrect = numericStat(session.interaction_stats, "incorrect_count", 0);
  const refusals = numericStat(session.refusal_stats, "refusal_count", 0);
  const anomalies = numericStat(session.anomaly_stats, "anomaly_flag_count", 0);
  const memoryReads = numericStat(session.memory_stats, "memory_reads", 0);
  const confirmed = numericStat(session.analysis_snapshot, "confirmed_count", 0);
  const boundary = numericStat(session.analysis_snapshot, "boundary_count", 0);
  const emergent = numericStat(session.analysis_snapshot, "emergent_count", 0);
  const domainCount = numericStat(session.curriculum_stats, "domain_count", 0);
  const conceptCount = numericStat(session.curriculum_stats, "concept_count", 0);
  const teacherLatency = session.teacher_snapshot?.latency_ms ?? 0;
  const runtimeReady = session.teacher_snapshot?.endpoint_ready;
  const accuracy = items > 0 ? `${Math.round((correct / items) * 100)}%` : "n/a";
  const note = session.notes?.[0] ?? "No session note recorded.";
  const statusLabel = session.completed_at ? "completed" : "in progress";
  const warningCount = session.curriculum_preview?.warnings?.length ?? 0;
  const artifactCount = session.artifacts?.length ?? 0;
  const skillProfile = formatSkillRunProfile(session.skill_run_snapshot);
  const approvedSkillCount = session.skill_run_snapshot?.approved_count ?? 0;
  const approvedSkillList = formatSkillRunApprovedList(session.skill_run_snapshot);
  const blockedSkillCount = formatSkillRunBlockedCount(session.skill_run_snapshot);
  const topArtifactLinks = (session.artifacts ?? [])
    .slice(0, 3)
    .map(
      (artifact) => `
        <a class="recent-link" href="${escapeHtml(artifact.download_path)}" target="_blank" rel="noreferrer">
          ${escapeHtml(artifact.label)}
        </a>
      `,
    )
    .join("");

  return `
    <article class="recent-card recent-card--${statusLabel.replace(" ", "-")}">
      <div class="recent-card-top">
        <div>
          <p class="recent-kicker">${isLatest ? "Latest session" : "Recent session"}</p>
          <h3>${escapeHtml(session.run_mode)} <span>${escapeHtml(session.run_id.slice(0, 8))}</span></h3>
        </div>
        <span class="recent-pill recent-pill--${statusLabel.replace(" ", "-")}">${escapeHtml(statusLabel)}</span>
      </div>
      <p class="recent-time">${escapeHtml(session.completed_at ?? "still running")}</p>
      <div class="recent-metric-grid">
        <div class="recent-metric">
          <span>Teacher</span>
          <strong>${escapeHtml(session.teacher_backend_id)}</strong>
        </div>
        <div class="recent-metric">
          <span>Skill profile</span>
          <strong>${escapeHtml(skillProfile)}</strong>
        </div>
        <div class="recent-metric">
          <span>Accuracy</span>
          <strong>${accuracy}</strong>
        </div>
        <div class="recent-metric">
          <span>Items / probes</span>
          <strong>${items} / ${probes}</strong>
        </div>
        <div class="recent-metric">
          <span>Correct / incorrect</span>
          <strong>${correct} / ${incorrect}</strong>
        </div>
        <div class="recent-metric">
          <span>Refusals / anomalies</span>
          <strong>${refusals} / ${anomalies}</strong>
        </div>
        <div class="recent-metric">
          <span>Signals</span>
          <strong>${confirmed} / ${boundary} / ${emergent}</strong>
        </div>
        <div class="recent-metric">
          <span>Teacher runtime</span>
          <strong>${teacherLatency} ms / ${runtimeReady === undefined ? "n/a" : runtimeReady ? "ready" : "not ready"}</strong>
        </div>
      </div>
      <div class="recent-section">
        <p>Curriculum:
          <strong>${domainCount} domains / ${conceptCount} concepts / ${items} items</strong>
        </p>
        <p>Item mix:
          <strong>${escapeHtml(formatCountMetrics(session.curriculum_preview?.item_mix ?? []))}</strong>
        </p>
        <p>Warnings:
          <strong>${warningCount}</strong>
        </p>
      </div>
      <div class="recent-section">
        <p>Approved skills:
          <strong>${approvedSkillCount} active</strong>
        </p>
        <p>Blocked skills:
          <strong>${escapeHtml(blockedSkillCount)}</strong>
        </p>
        <p>Signals:
          <strong>${confirmed} confirmed / ${boundary} boundary / ${emergent} emergent</strong>
        </p>
        <p>Memory reads:
          <strong>${memoryReads}</strong>
        </p>
        <p>Artifacts captured:
          <strong>${artifactCount}</strong>
        </p>
      </div>
      <details class="recent-details" data-persist-key="recent-session-${escapeHtml(session.run_id)}">
        <summary data-tooltip="Expand to view the session note, full approved skill set, and session root path.">More details</summary>
        <div class="recent-section recent-section--details">
          <p>Note: <strong>${escapeHtml(note)}</strong></p>
          <p>Approved skill set: <strong>${escapeHtml(approvedSkillList)}</strong></p>
          <p>Session root: <code>${escapeHtml(session.root_dir)}</code></p>
        </div>
      </details>
      <div class="recent-links">
        ${topArtifactLinks || '<span class="recent-link-placeholder">No artifact shortcuts.</span>'}
      </div>
    </article>
  `;
}

function renderStringList(items, emptyMessage) {
  if (!items?.length) {
    return `<p>${escapeHtml(emptyMessage)}</p>`;
  }

  return `
    <ul>
      ${items.map((item) => `<li>${escapeHtml(item)}</li>`).join("")}
    </ul>
  `;
}

function renderTelemetryPreview(rows) {
  if (!rows.length) {
    return "<p>No telemetry preview rows are available for this session yet.</p>";
  }

  return `
    <div class="telemetry-list">
      ${rows
        .map(
          (row) => `
            <article class="telemetry-entry">
              <p><strong>${escapeHtml(row.item_id)}</strong> <span>${escapeHtml(row.item_type)}</span></p>
              <p>Prompt: <strong>${escapeHtml(truncateText(row.prompt, 180))}</strong></p>
              <p>Expected / Janet:
                <strong>${escapeHtml(row.expected_answer ?? "n/a")} / ${escapeHtml(row.janet_answer ?? "refused")}</strong>
              </p>
              <p>Correctness / fit / mode:
                <strong>${escapeHtml(row.correctness_judgment)} / ${escapeHtml(row.structure_fit)} / ${escapeHtml(row.final_mode ?? "n/a")}</strong>
              </p>
              <p>Executed skill: <strong>${escapeHtml(row.executed_skill ?? "n/a")}</strong></p>
              <p>Memory reads: <strong>${escapeHtml(formatListInline(row.memory_reads))}</strong></p>
              <p>Anomalies: <strong>${escapeHtml(formatListInline(row.anomaly_flags))}</strong></p>
              <p>Refusal reason: <strong>${escapeHtml(row.refusal_reason ?? "n/a")}</strong></p>
              <details class="telemetry-details" data-persist-key="telemetry-${escapeHtml(row.item_id)}">
                <summary data-tooltip="Expand to inspect candidate skills, policy checks, reasoning steps, and other trace-level detail for this item.">Trace details</summary>
                <div class="telemetry-details__body">
                  <p>Candidate / approved / blocked:
                    <strong>${escapeHtml(formatListInline(row.candidate_skills))}</strong> /
                    <strong>${escapeHtml(formatListInline(row.approved_skills))}</strong> /
                    <strong>${escapeHtml(formatListInline(row.blocked_skills))}</strong>
                  </p>
                  <p>Policy checks: <strong>${escapeHtml(formatListInline(row.policy_checks))}</strong></p>
                  <p>Reasoning steps: <strong>${escapeHtml(formatListInline(row.reasoning_steps))}</strong></p>
                  ${row.refusal_next_steps?.length
                    ? `<p>Next steps: <strong>${escapeHtml(formatListInline(row.refusal_next_steps))}</strong></p>`
                    : ""}
                  ${row.anomaly_explanation
                    ? `<p>Anomaly note: <strong>${escapeHtml(truncateText(row.anomaly_explanation, 180))}</strong></p>`
                    : ""}
                </div>
              </details>
            </article>
          `,
        )
        .join("")}
    </div>
  `;
}

function formatListInline(items) {
  if (!items?.length) return "n/a";
  return items.join(", ");
}

function formatCountMetrics(metrics) {
  if (!metrics?.length) return "n/a";
  return metrics.map((entry) => `${entry.label}: ${entry.count}`).join(", ");
}

function numericStat(source, key, fallback = 0) {
  const value = source?.[key];
  return typeof value === "number" ? value : fallback;
}

function setStatus(message, tone) {
  UI_STATE.statusMessage = message;
  UI_STATE.statusTone = tone;
}

function syncRenderedStatus() {
  const status = document.getElementById("command-status");
  if (!status) return;
  status.textContent = UI_STATE.statusMessage;
  status.className = `control-status control-status--${UI_STATE.statusTone}`;
}

function isInteractiveEditing() {
  const active = document.activeElement;
  if (!active) return false;

  if (active.matches("input, select, textarea")) {
    return true;
  }

  if (active.closest?.("[data-tooltip]") && active.matches("summary")) {
    return false;
  }

  return Boolean(active.closest?.("#control-card, #curriculum-card, #compare-sessions"));
}

function truncateText(value, maxLength) {
  if (value.length <= maxLength) return value;
  return `${value.slice(0, maxLength - 3)}...`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function formatJobProgress(progress) {
  if (!progress) return "n/a";
  return `${progress.phase} ${progress.total_items_completed}/${progress.total_items_expected ?? "?"}`;
}

function isActionDisabled(actionId, actionRunning, hasActiveJob, activeBridgeState) {
  if (actionId === "update_skill_approvals") {
    return actionRunning || hasActiveJob;
  }
  if (actionId === "stop_run") {
    return !hasActiveJob || ["completed", "stopped", "failed"].includes(activeBridgeState);
  }
  if (actionId === "pause_run") {
    return !hasActiveJob || !["queued", "running", "pausing"].includes(activeBridgeState);
  }
  if (actionId === "resume_run") {
    return !hasActiveJob || !["paused", "pausing"].includes(activeBridgeState);
  }
  return actionRunning || hasActiveJob;
}

function ensureSelectedSkills(skillSnapshot) {
  if (!skillSnapshot?.entries?.length) {
    UI_STATE.selectedSkillIds = [];
    return;
  }

  const validIds = new Set(skillSnapshot.entries.map((entry) => entry.skill_id));
  if (!UI_STATE.selectedSkillIds) {
    UI_STATE.selectedSkillIds = skillSnapshot.entries
      .filter((entry) => entry.approved)
      .map((entry) => entry.skill_id);
    return;
  }

  UI_STATE.selectedSkillIds = UI_STATE.selectedSkillIds.filter((skillId) => validIds.has(skillId));
}

function hasSkillSelectionChanges(skillSnapshot) {
  const approved = (skillSnapshot?.entries ?? [])
    .filter((entry) => entry.approved)
    .map((entry) => entry.skill_id)
    .sort();
  const selected = [...(UI_STATE.selectedSkillIds ?? [])].sort();
  return approved.length !== selected.length || approved.some((skillId, index) => skillId !== selected[index]);
}

function formatSkillRunProfile(snapshot) {
  if (!snapshot) return "not recorded";
  if (snapshot.selection_mode === "memory_only") return "memory only (0 skills approved)";
  if (snapshot.selection_mode === "all_skills") {
    return `all skills (${snapshot.approved_count}/${snapshot.total_skill_count})`;
  }
  return `restricted (${snapshot.approved_count}/${snapshot.total_skill_count})`;
}

function formatSkillRunApprovedList(snapshot) {
  if (!snapshot) return "not recorded";
  if (!snapshot.approved_skill_ids?.length) return "memory only";
  return formatListInline(snapshot.approved_skill_ids);
}

function formatSkillRunBlockedCount(snapshot) {
  if (!snapshot) return "not recorded";
  return String(snapshot.blocked_skill_ids?.length ?? 0);
}

function accuracyValue(session) {
  const total = numericStat(session.interaction_stats, "total_items", 0);
  const correct = numericStat(session.interaction_stats, "correct_count", 0);
  if (!total) return 0;
  return Math.round((correct / total) * 100);
}

function formatAccuracy(session) {
  const total = numericStat(session.interaction_stats, "total_items", 0);
  return total ? `${accuracyValue(session)}%` : "n/a";
}

function formatDelta(value, suffix = "") {
  const normalized = Number.isFinite(value) ? value : 0;
  const prefix = normalized > 0 ? "+" : "";
  return `${prefix}${normalized}${suffix}`;
}

function compareSkillLists(primarySkills, secondarySkills) {
  const primaryOnly = primarySkills.filter((skill) => !secondarySkills.includes(skill));
  const secondaryOnly = secondarySkills.filter((skill) => !primarySkills.includes(skill));

  if (!primaryOnly.length && !secondaryOnly.length) {
    return "same approved set";
  }

  const parts = [];
  if (primaryOnly.length) {
    parts.push(`primary-only: ${primaryOnly.join(", ")}`);
  }
  if (secondaryOnly.length) {
    parts.push(`secondary-only: ${secondaryOnly.join(", ")}`);
  }
  return parts.join(" | ");
}

function applyComparePreset(presetId) {
  switch (presetId) {
    case "changed_refusals":
      UI_STATE.compareShowChangedOnly = true;
      UI_STATE.compareShowRefusalsOnly = true;
      UI_STATE.compareShowAnomaliesOnly = false;
      UI_STATE.compareSkillShiftOnly = false;
      UI_STATE.compareDomainFilter = "all";
      UI_STATE.compareItemTypeFilter = "all";
      break;
    case "anomaly_shifts":
      UI_STATE.compareShowChangedOnly = true;
      UI_STATE.compareShowRefusalsOnly = false;
      UI_STATE.compareShowAnomaliesOnly = true;
      UI_STATE.compareSkillShiftOnly = false;
      UI_STATE.compareDomainFilter = "all";
      UI_STATE.compareItemTypeFilter = "all";
      break;
    case "skill_shifts":
      UI_STATE.compareShowChangedOnly = true;
      UI_STATE.compareShowRefusalsOnly = false;
      UI_STATE.compareShowAnomaliesOnly = false;
      UI_STATE.compareSkillShiftOnly = true;
      UI_STATE.compareDomainFilter = "all";
      UI_STATE.compareItemTypeFilter = "all";
      break;
    default:
      UI_STATE.compareShowChangedOnly = true;
      UI_STATE.compareShowRefusalsOnly = false;
      UI_STATE.compareShowAnomaliesOnly = false;
      UI_STATE.compareSkillShiftOnly = false;
      UI_STATE.compareDomainFilter = "all";
      UI_STATE.compareItemTypeFilter = "all";
      break;
  }
}

function buildCompareFilterOptions(primary, secondary) {
  const domains = new Set();
  const itemTypes = new Set();

  [...(primary.comparison_items ?? []), ...(secondary.comparison_items ?? [])].forEach((item) => {
    domains.add(inferDomainIdFromItemId(item.item_id));
    itemTypes.add(item.item_type);
  });

  return {
    domains: [...domains].filter(Boolean).sort(),
    itemTypes: [...itemTypes].filter(Boolean).sort(),
  };
}

function inferDomainIdFromItemId(itemId) {
  if (!itemId) return "unknown";
  return itemId.split("-")[0] ?? "unknown";
}

function buildCompareContext(primary, secondary) {
  const primaryItems = new Map((primary.comparison_items ?? []).map((item) => [item.item_id, item]));
  const secondaryItems = new Map((secondary.comparison_items ?? []).map((item) => [item.item_id, item]));
  const overlapping = [...primaryItems.keys()]
    .filter((itemId) => secondaryItems.has(itemId))
    .map((itemId) => {
      const primaryItem = primaryItems.get(itemId);
      const secondaryItem = secondaryItems.get(itemId);
      const domainId = inferDomainIdFromItemId(itemId);
      return {
        itemId,
        domainId,
        primaryItem,
        secondaryItem,
        changed:
          (primaryItem.janet_answer ?? null) !== (secondaryItem.janet_answer ?? null)
          || primaryItem.correctness_judgment !== secondaryItem.correctness_judgment
          || primaryItem.structure_fit !== secondaryItem.structure_fit
          || (primaryItem.executed_skill ?? null) !== (secondaryItem.executed_skill ?? null)
          || (primaryItem.refusal_reason ?? null) !== (secondaryItem.refusal_reason ?? null)
          || formatListInline(primaryItem.anomaly_flags) !== formatListInline(secondaryItem.anomaly_flags),
        hasRefusal: Boolean(primaryItem.refusal_reason || secondaryItem.refusal_reason),
        hasAnomaly: Boolean((primaryItem.anomaly_flags?.length ?? 0) || (secondaryItem.anomaly_flags?.length ?? 0)),
        hasSkillShift: (primaryItem.executed_skill ?? null) !== (secondaryItem.executed_skill ?? null),
      };
    });
  const filtered = overlapping.filter((entry) => {
    if (UI_STATE.compareShowChangedOnly && !entry.changed) return false;
    if (UI_STATE.compareShowRefusalsOnly && !entry.hasRefusal) return false;
    if (UI_STATE.compareShowAnomaliesOnly && !entry.hasAnomaly) return false;
    if (UI_STATE.compareSkillShiftOnly && !entry.hasSkillShift) return false;
    if (UI_STATE.compareDomainFilter !== "all" && entry.domainId !== UI_STATE.compareDomainFilter) return false;
    if (
      UI_STATE.compareItemTypeFilter !== "all"
      && entry.primaryItem.item_type !== UI_STATE.compareItemTypeFilter
      && entry.secondaryItem.item_type !== UI_STATE.compareItemTypeFilter
    ) {
      return false;
    }
    return true;
  });

  return {
    overlapping,
    filtered,
    visible: filtered.slice(0, 12),
  };
}

async function saveCompareExport(format, primary, secondary) {
  const compareContext = buildCompareContext(primary, secondary);
  const exportItems = UI_STATE.compareExportScope === "all_filtered"
    ? compareContext.filtered
    : compareContext.visible;
  const payload = format === "json"
    ? buildCompareExportJson(primary, secondary, compareContext, exportItems)
    : buildCompareExportMarkdown(primary, secondary, compareContext, exportItems);
  const extension = format === "json" ? "json" : "md";
  const contentType = format === "json" ? "application/json; charset=utf-8" : "text/markdown; charset=utf-8";
  const fileName = buildCompareExportFileName(primary, secondary, UI_STATE.compareExportScope, extension);

  if (window.__JANET_BRIDGE__?.saveCompareExport) {
    try {
      const saved = await window.__JANET_BRIDGE__.saveCompareExport({
        file_name: fileName,
        content: payload,
        content_type: contentType,
      });
      UI_STATE.lastCompareExport = saved;
      setStatus(
        `Saved compare export to ${saved.relative_path}.`,
        "success",
      );
      document.getElementById("compare-sessions").innerHTML = renderCompareSessions(window.__JANET_SCHOOL__?.recent_sessions ?? []);
      bindCompareSessions(window.__JANET_SCHOOL__?.recent_sessions ?? []);
      syncRenderedStatus();
      return;
    } catch (error) {
      setStatus(`Workspace compare export save failed: ${error.message}`, "error");
      syncRenderedStatus();
      return;
    }
  }

  downloadTextFile(fileName, payload, contentType);
  setStatus(
    `Downloaded compare export for ${primary.run_id.slice(0, 8)} vs ${secondary.run_id.slice(0, 8)}.`,
    "success",
  );
  syncRenderedStatus();
}

function buildCompareExportJson(primary, secondary, compareContext, exportItems) {
  const exportPayload = {
    exported_at: new Date().toISOString(),
    primary_run: buildCompareRunSummary(primary),
    secondary_run: buildCompareRunSummary(secondary),
    filters: buildCompareFilterSnapshot(),
    delta_summary: buildCompareDeltaList(primary, secondary),
    overlap_counts: {
      overlapping_items: compareContext.overlapping.length,
      filtered_items: compareContext.filtered.length,
      exported_item_count: exportItems.length,
    },
    exported_overlap_items: exportItems.map((entry) => ({
      item_id: entry.itemId,
      domain_id: entry.domainId,
      item_type: entry.primaryItem.item_type,
      changed: entry.changed,
      has_refusal: entry.hasRefusal,
      has_anomaly: entry.hasAnomaly,
      has_skill_shift: entry.hasSkillShift,
      prompt: entry.primaryItem.prompt,
      expected_answer: entry.primaryItem.expected_answer ?? null,
      primary: {
        janet_answer: entry.primaryItem.janet_answer ?? null,
        correctness_judgment: entry.primaryItem.correctness_judgment,
        structure_fit: entry.primaryItem.structure_fit,
        executed_skill: entry.primaryItem.executed_skill ?? null,
        refusal_reason: entry.primaryItem.refusal_reason ?? null,
        anomaly_flags: entry.primaryItem.anomaly_flags ?? [],
      },
      secondary: {
        janet_answer: entry.secondaryItem.janet_answer ?? null,
        correctness_judgment: entry.secondaryItem.correctness_judgment,
        structure_fit: entry.secondaryItem.structure_fit,
        executed_skill: entry.secondaryItem.executed_skill ?? null,
        refusal_reason: entry.secondaryItem.refusal_reason ?? null,
        anomaly_flags: entry.secondaryItem.anomaly_flags ?? [],
      },
    })),
  };

  return JSON.stringify(exportPayload, null, 2);
}

function buildCompareExportMarkdown(primary, secondary, compareContext, exportItems) {
  const filters = buildCompareFilterSnapshot();
  const deltaLines = buildCompareDeltaList(primary, secondary);
  const exportedEntries = exportItems.length
    ? exportItems.map((entry) => {
      const flags = [
        entry.changed ? "changed" : "same",
        entry.hasRefusal ? "refusal" : "no refusal",
        entry.hasAnomaly ? "anomaly" : "no anomaly",
        entry.hasSkillShift ? "skill shift" : "same skill path",
      ].join(", ");

      return [
        `## ${entry.itemId}`,
        ``,
        `- Domain: ${entry.domainId}`,
        `- Item type: ${entry.primaryItem.item_type}`,
        `- Flags: ${flags}`,
        `- Prompt: ${entry.primaryItem.prompt}`,
        `- Expected answer: ${entry.primaryItem.expected_answer ?? "n/a"}`,
        `- Primary: ${(entry.primaryItem.janet_answer ?? "refused")} | ${entry.primaryItem.correctness_judgment} | ${entry.primaryItem.executed_skill ?? "n/a"}`,
        `- Primary fit/refusal/anomalies: ${entry.primaryItem.structure_fit} | ${entry.primaryItem.refusal_reason ?? "n/a"} | ${formatListInline(entry.primaryItem.anomaly_flags)}`,
        `- Secondary: ${(entry.secondaryItem.janet_answer ?? "refused")} | ${entry.secondaryItem.correctness_judgment} | ${entry.secondaryItem.executed_skill ?? "n/a"}`,
        `- Secondary fit/refusal/anomalies: ${entry.secondaryItem.structure_fit} | ${entry.secondaryItem.refusal_reason ?? "n/a"} | ${formatListInline(entry.secondaryItem.anomaly_flags)}`,
        ``,
      ].join("\n");
    }).join("\n")
    : "No overlap items matched the current compare filters.";

  return [
    "# Janet School Compare Export",
    "",
    `Exported at: ${new Date().toISOString()}`,
    "",
    "## Run Pair",
    "",
    `- Primary: ${primary.run_id} (${formatSkillRunProfile(primary.skill_run_snapshot)})`,
    `- Secondary: ${secondary.run_id} (${formatSkillRunProfile(secondary.skill_run_snapshot)})`,
    `- Primary approved skills: ${formatSkillRunApprovedList(primary.skill_run_snapshot)}`,
    `- Secondary approved skills: ${formatSkillRunApprovedList(secondary.skill_run_snapshot)}`,
    "",
    "## Filters",
    "",
    `- Changed only: ${filters.changed_only}`,
    `- Refusals only: ${filters.refusals_only}`,
    `- Anomalies only: ${filters.anomalies_only}`,
    `- Skill shifts only: ${filters.skill_shifts_only}`,
    `- Domain filter: ${filters.domain}`,
    `- Item type filter: ${filters.item_type}`,
    "",
    "## Delta Summary",
    "",
    ...deltaLines.map((line) => `- ${line}`),
    "",
    "## Overlap Counts",
    "",
    `- Overlapping items: ${compareContext.overlapping.length}`,
    `- Matching current filters: ${compareContext.filtered.length}`,
    `- Exported item count: ${exportItems.length}`,
    "",
    `## Exported Items (${UI_STATE.compareExportScope === "all_filtered" ? "All Filtered" : "Visible"})`,
    "",
    exportedEntries,
    "",
  ].join("\n");
}

function buildCompareExportFileName(primary, secondary, scope, extension) {
  const timestamp = new Date().toISOString().replaceAll(":", "").replaceAll("-", "").replace(/\..+$/, "Z");
  return `janet-compare-${primary.run_id.slice(0, 8)}-vs-${secondary.run_id.slice(0, 8)}-${scope}-${timestamp}.${extension}`;
}

function renderLastCompareExport() {
  const saved = UI_STATE.lastCompareExport;
  if (!saved) {
    return `
      <p class="control-note">Saved compare exports will appear here after the first workspace write.</p>
    `;
  }

  return `
    <div class="compare-export-result">
      <p>Latest saved compare export:
        <a class="artifact-link" href="${escapeHtml(saved.download_path)}" target="_blank" rel="noreferrer">${escapeHtml(saved.file_name)}</a>
      </p>
      <p class="artifact-meta">${escapeHtml(saved.relative_path)} | ${escapeHtml(saved.absolute_path)}</p>
    </div>
  `;
}

function buildCompareRunSummary(session) {
  return {
    run_id: session.run_id,
    completed_at: session.completed_at ?? null,
    teacher_backend_id: session.teacher_backend_id,
    skill_profile: formatSkillRunProfile(session.skill_run_snapshot),
    approved_skill_ids: session.skill_run_snapshot?.approved_skill_ids ?? [],
    blocked_skill_ids: session.skill_run_snapshot?.blocked_skill_ids ?? [],
    accuracy: formatAccuracy(session),
    totals: {
      total_items: numericStat(session.interaction_stats, "total_items", 0),
      probe_count: numericStat(session.interaction_stats, "probe_count", 0),
      refusal_count: numericStat(session.refusal_stats, "refusal_count", 0),
      anomaly_flag_count: numericStat(session.anomaly_stats, "anomaly_flag_count", 0),
      confirmed_count: numericStat(session.analysis_snapshot, "confirmed_count", 0),
      boundary_count: numericStat(session.analysis_snapshot, "boundary_count", 0),
      emergent_count: numericStat(session.analysis_snapshot, "emergent_count", 0),
    },
  };
}

function buildCompareFilterSnapshot() {
  return {
    changed_only: UI_STATE.compareShowChangedOnly,
    refusals_only: UI_STATE.compareShowRefusalsOnly,
    anomalies_only: UI_STATE.compareShowAnomaliesOnly,
    skill_shifts_only: UI_STATE.compareSkillShiftOnly,
    domain: UI_STATE.compareDomainFilter,
    item_type: UI_STATE.compareItemTypeFilter,
  };
}

function buildCompareDeltaList(primary, secondary) {
  return [
    `Skill profile: ${formatSkillRunProfile(primary.skill_run_snapshot)} vs ${formatSkillRunProfile(secondary.skill_run_snapshot)}.`,
    `Approved skill count delta: ${formatDelta((primary.skill_run_snapshot?.approved_count ?? 0) - (secondary.skill_run_snapshot?.approved_count ?? 0))}.`,
    `Accuracy delta: ${formatDelta(accuracyValue(primary) - accuracyValue(secondary), "%")}.`,
    `Refusal delta: ${formatDelta(numericStat(primary.refusal_stats, "refusal_count", 0) - numericStat(secondary.refusal_stats, "refusal_count", 0))}.`,
    `Anomaly delta: ${formatDelta(numericStat(primary.anomaly_stats, "anomaly_flag_count", 0) - numericStat(secondary.anomaly_stats, "anomaly_flag_count", 0))}.`,
    `Confirmed signal delta: ${formatDelta(numericStat(primary.analysis_snapshot, "confirmed_count", 0) - numericStat(secondary.analysis_snapshot, "confirmed_count", 0))}.`,
    `Boundary signal delta: ${formatDelta(numericStat(primary.analysis_snapshot, "boundary_count", 0) - numericStat(secondary.analysis_snapshot, "boundary_count", 0))}.`,
    `Emergent signal delta: ${formatDelta(numericStat(primary.analysis_snapshot, "emergent_count", 0) - numericStat(secondary.analysis_snapshot, "emergent_count", 0))}.`,
    `Approved skill set difference: ${compareSkillLists(primary.skill_run_snapshot?.approved_skill_ids ?? [], secondary.skill_run_snapshot?.approved_skill_ids ?? [])}.`,
  ];
}

function downloadTextFile(fileName, text, mimeType) {
  const blob = new Blob([text], { type: `${mimeType};charset=utf-8` });
  const objectUrl = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = objectUrl;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
}

bootstrapStartupSplash();
loadGuiState();
