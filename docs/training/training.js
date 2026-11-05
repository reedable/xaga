const element = (id) => document.getElementById(id);
const storageKey = "xaga-training-v1";
const levels = ["Introduction", "Applied", "Advanced"];
let progress = {};
let active;
let sourceRequest;
let sourceOpener;

function validProgress(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const clean = {};
  for (const [trackId, track] of Object.entries(TRAINING)) {
    for (const lesson of track.lessons) {
      const key = `${trackId}/${lesson.id}`;
      if (Array.isArray(value[key])) {
        clean[key] = [...new Set(value[key].filter((index) => Number.isInteger(index) && index >= 0 && index < lesson.questions.length))];
      }
    }
  }
  return clean;
}
try {
  progress = validProgress(JSON.parse(localStorage.getItem(storageKey)));
} catch {
  element("storage-note").textContent = "Progress is kept for this visit; saved browser storage is unavailable.";
}
function saveProgress() {
  try { localStorage.setItem(storageKey, JSON.stringify(progress)); }
  catch { element("storage-note").textContent = "Progress is kept for this visit; saved browser storage is unavailable."; }
}
function isComplete(trackId, lesson) {
  return progress[`${trackId}/${lesson.id}`]?.length === lesson.questions.length;
}
function updateProgress() {
  for (const [trackId, track] of Object.entries(TRAINING)) {
    const done = track.lessons.filter((lesson) => isComplete(trackId, lesson)).length;
    document.querySelector(`[data-track="${trackId}"] .track-status`).textContent = `${done} / ${track.lessons.length} lessons completed →`;
  }
  if (!active) return;
  const { trackId, lesson, track } = active;
  const count = progress[`${trackId}/${lesson.id}`]?.length || 0;
  element("completion").textContent = isComplete(trackId, lesson) ? "Lesson completed ✓" : `${count} / ${lesson.questions.length} checks completed`;
  element("track-progress").textContent = `${track.lessons.filter((item) => isComplete(trackId, item)).length} / ${track.lessons.length} lessons completed`;
  document.querySelectorAll("[data-lesson-status]").forEach((node) => {
    const item = track.lessons.find((candidate) => candidate.id === node.dataset.lessonStatus);
    node.textContent = isComplete(trackId, item) ? "Completed" : "";
  });
}
function paragraph(text) {
  const node = document.createElement("p");
  node.textContent = text;
  return node;
}
function renderCurriculum() {
  const { trackId, track, lesson } = active;
  element("curriculum").replaceChildren(...levels.map((level) => {
    const section = document.createElement("div");
    const heading = document.createElement("h3");
    heading.textContent = level;
    const list = document.createElement("ol");
    list.start = track.lessons.findIndex((item) => item.level === level) + 1;
    for (const item of track.lessons.filter((candidate) => candidate.level === level)) {
      const row = document.createElement("li");
      const link = document.createElement("a");
      link.href = `#${trackId}/${item.id}`;
      link.textContent = item.title;
      if (item.id === lesson.id) link.setAttribute("aria-current", "step");
      const status = document.createElement("span");
      status.className = "lesson-status";
      status.dataset.lessonStatus = item.id;
      link.append(status);
      row.append(link);
      list.append(row);
    }
    section.append(heading, list);
    return section;
  }));
}
function renderQuestion(question, questionIndex) {
  const { trackId, lesson, index: lessonIndex } = active;
  const key = `${trackId}/${lesson.id}`;
  const form = document.createElement("form");
  const fieldset = document.createElement("fieldset");
  const legend = document.createElement("legend");
  legend.textContent = `${questionIndex + 1}. ${question.prompt}`;
  fieldset.append(legend);
  // Stable rotation keeps correct answers in different positions on each lesson.
  const offset = (lessonIndex + questionIndex) % question.answers.length;
  const options = question.answers.map((_, index) => question.answers[(index + offset) % question.answers.length]);
  for (const [index, answer] of options.entries()) {
    const label = document.createElement("label");
    label.className = "answer";
    const input = document.createElement("input");
    input.type = "radio";
    input.name = "answer";
    input.value = index;
    input.required = true;
    label.append(input, document.createTextNode(answer.text));
    fieldset.append(label);
  }
  const submit = document.createElement("button");
  submit.type = "submit";
  submit.textContent = "Check answer";
  const feedback = document.createElement("div");
  feedback.className = "feedback";
  feedback.setAttribute("role", "status");
  feedback.setAttribute("aria-live", "polite");
  feedback.setAttribute("aria-atomic", "true");
  if (progress[key]?.includes(questionIndex)) {
    feedback.dataset.correct = "true";
    feedback.textContent = "Previously completed. Try again to revisit your reasoning.";
  }
  form.addEventListener("change", () => {
    feedback.textContent = "";
    delete feedback.dataset.correct;
  });
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    const selected = new FormData(form).get("answer");
    if (selected === null) return;
    const answer = options[Number(selected)];
    feedback.dataset.correct = String(answer.correct);
    feedback.textContent = `${answer.correct ? "Correct. " : "Try again. "}${answer.feedback}`;
    if (answer.correct) {
      progress[key] = [...new Set([...(progress[key] || []), questionIndex])];
      saveProgress();
      updateProgress();
    }
  });
  form.append(fieldset, submit, feedback);
  return form;
}
function sourceLink(reference) {
  const row = document.createElement("li");
  const preview = document.createElement("button");
  preview.type = "button";
  preview.className = "text-button";
  preview.textContent = reference.title;
  preview.addEventListener("click", () => openSource(reference, preview));
  const original = document.createElement("a");
  original.href = reference.path;
  original.target = "_blank";
  original.rel = "noopener";
  original.textContent = "Open file ↗";
  original.setAttribute("aria-label", `Open ${reference.title} in a new tab`);
  row.append(preview, original);
  return row;
}
async function openSource(reference, opener) {
  sourceRequest?.abort();
  const controller = new AbortController();
  sourceRequest = controller;
  sourceOpener = opener;
  const dialog = element("source-dialog");
  element("source-title").textContent = reference.title;
  element("source-original").href = reference.path;
  element("source-content").textContent = "";
  if (!dialog.open) dialog.showModal();
  element("close-source").focus();
  if (location.protocol === "file:") {
    element("source-status").textContent = "Use Open source in a new tab to read this file. For in-page source reading, serve the repository locally as described in the training README.";
    return;
  }
  element("source-status").textContent = "Loading source…";
  try {
    const response = await fetch(reference.path.split("#")[0], { signal: controller.signal });
    if (!response.ok) throw new Error("Source unavailable");
    const text = await response.text();
    if (controller.signal.aborted) return;
    element("source-content").textContent = text;
    element("source-status").textContent = "Source text · Use browser find to locate symbols or headings.";
  } catch {
    if (!controller.signal.aborted) element("source-status").textContent = "Source preview is unavailable. Try opening the source file in a new tab.";
  }
}
element("close-source").addEventListener("click", () => element("source-dialog").close());
element("source-dialog").addEventListener("close", () => {
  sourceRequest?.abort();
  if (sourceOpener?.isConnected) sourceOpener.focus();
});
function navigate(focus = true) {
  const [trackId, lessonId] = location.hash.slice(1).split("/");
  const track = Object.hasOwn(TRAINING, trackId) ? TRAINING[trackId] : null;
  const index = track ? (lessonId ? track.lessons.findIndex((item) => item.id === lessonId) : 0) : -1;
  element("course").hidden = !track || index < 0;
  document.querySelectorAll("[data-track]").forEach((link) => {
    if (track && link.dataset.track === trackId) link.setAttribute("aria-current", "true");
    else link.removeAttribute("aria-current");
  });
  if (element("source-dialog").open) element("source-dialog").close();
  active = null;
  element("route-message").textContent = "";
  if (!track || index < 0) {
    document.title = "Xaga · Training";
    if (trackId) element("route-message").textContent = "That lesson could not be found. Choose a track above to continue.";
    if (focus) element("welcome").focus();
    return;
  }
  const lesson = track.lessons[index];
  active = { trackId, track, lesson, index };
  document.title = `${lesson.title} · Xaga Training`;
  element("course-title").textContent = track.title;
  element("lesson-meta").textContent = `${lesson.level} / Lesson ${index + 1} of ${track.lessons.length} · About 5–8 minutes`;
  element("lesson-title").textContent = lesson.title;
  element("objective").textContent = lesson.objective;
  element("explanation").replaceChildren(...lesson.paragraphs.map(paragraph));
  element("example").textContent = lesson.example;
  element("references").replaceChildren(...lesson.references.map(sourceLink));
  element("quizzes").replaceChildren(...lesson.questions.map(renderQuestion));
  renderCurriculum();
  updateProgress();
  const previous = element("previous");
  previous.hidden = index === 0;
  if (index > 0) {
    previous.href = `#${trackId}/${track.lessons[index - 1].id}`;
    previous.textContent = `← ${track.lessons[index - 1].title}`;
  }
  const next = element("next");
  next.href = index + 1 < track.lessons.length ? `#${trackId}/${track.lessons[index + 1].id}` : "#";
  next.textContent = index + 1 < track.lessons.length ? `${track.lessons[index + 1].title} →` : "Explore another track →";
  if (focus) element("lesson-title").focus();
}
element("reset-progress").addEventListener("click", () => {
  progress = {};
  saveProgress();
  navigate(false);
  updateProgress();
  element("progress-message").textContent = "Training progress reset.";
});
document.querySelector(".skip-link").addEventListener("click", (event) => {
  event.preventDefault();
  element("welcome").focus();
});
window.addEventListener("hashchange", () => navigate());
updateProgress();
navigate(Boolean(location.hash));
