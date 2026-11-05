const lessons = {
  product: {
    label: "Product", title: "A bookmark is a color", minutes: 3,
    paragraphs: [
      "Xaga’s organization model treats bookmarking an article as assigning a color from the annotation palette. Removing the color removes the bookmark. There is no separate stored or derived bookmark boolean.",
      "This connects the user action to a single domain value: an article can be unmarked or marked with one of six colors. A bookmarked article remains eligible for bookmark export even when the export format cannot carry its color.",
      "For product and UX work, this matters when describing actions and states. Recoloring changes the bookmark’s color while the article remains bookmarked. Removing a bookmark clears the color."
    ],
    question: "A trainee changes a bookmarked article from Blue to Violet. What should that mean?",
    answers: [
      ["The article remains bookmarked, now with Violet as its color.", true, "Correct. The color is the bookmark state. Recoloring preserves membership among bookmarked articles."],
      ["The old bookmark is removed, leaving the article unbookmarked.", false, "Violet is still a bookmark color. Only removing the color removes the bookmark."],
      ["A separate bookmark flag must be turned on to keep it bookmarked.", false, "Xaga has no separate bookmark boolean. Introducing one would duplicate the state already expressed by color."]
    ],
    references: [["Domain: reading state, bookmarks, and color", "../architecture/organization_model.md#reading-state-bookmarks-and-color"], ["Code: add_bookmark and remove_bookmark", "../../src-tauri/src/commands/bookmarks.rs"]]
  },
  interface: {
    label: "Interface", title: "Start with a complete static state", minutes: 3,
    paragraphs: [
      "Xaga’s motion convention begins with a usable interface that does not animate. Animation is added only inside a prefers-reduced-motion: no-preference media query. If the user requests reduced motion, or the media feature is unsupported, the static state remains.",
      "The content toolbar applies this to its reload icon: the base styles disable animation, and the media query adds a spin to the busy icon. This is a progressive enhancement of an already usable state.",
      "Motion cannot be the only signal that an operation is running. Static text and screen-reader feedback must communicate the state independently. The same convention applies to transitions and component-library motion."
    ],
    question: "You are adding a busy animation to a Xaga control. Which approach follows the convention?",
    answers: [
      ["Animate by default and disable animation only when reduced motion is requested.", false, "That leaves animation enabled when the media feature is unsupported. Xaga starts with no animation and opts in only for no-preference."],
      ["Provide static status feedback, then enable animation inside no-preference.", true, "Correct. The base state works without motion; animation is added only when the media query permits it."],
      ["Use a spinning icon as the only indication that work is in progress.", false, "A motion-only signal disappears in the static experience. Provide meaningful static text and screen-reader feedback too."]
    ],
    references: [["Convention: motion", "../conventions/motion.md"], ["Code: toolbar ReloadIconBusy styles", "../../src/components/content/Toolbar.module.css"]]
  },
  engineering: {
    label: "Engineering", title: "Carry the action across the boundary", minutes: 4,
    paragraphs: [
      "Xaga separates file logic from orchestration. Core owns pure logic and file APIs; the Tauri command layer sequences writes and enqueues work for the embedding service.",
      "The bookmark commands illustrate the boundary. After the writer successfully changes the article’s color, the command enqueues a typed SyncState job carrying the committed color. The consumer receives the intent without rediscovering it from a file change.",
      "A generic filesystem event would lose that knowledge. External edits have no command carrying intent and are reconciled by recovery. They are a different path from an interactive command."
    ],
    question: "An interactive command successfully changes an article’s color. Where should its SyncState job be enqueued?",
    answers: [
      ["In core’s file-writing logic, which should own the embedding queue.", false, "Core does not own queue orchestration. Putting it there would cross the documented layer boundary."],
      ["In a file watcher that infers the user action from the changed file.", false, "The command already knows the action. Inferring it again from a generic event discards intent and introduces an unnecessary round trip."],
      ["In the command layer, after the successful write, carrying the committed color.", true, "Correct. The command owns write-then-enqueue sequencing and passes typed state to downstream consumers."]
    ],
    references: [["Architecture: layer ownership", "../architecture/layer_ownership.md"], ["Principle: carry intent end-to-end", "../principles/carry_intent_end_to_end.md"], ["Code: bookmark commands and SyncState", "../../src-tauri/src/commands/bookmarks.rs"]]
  }
};

const completed = new Set();
let activeTrack;
const element = (id) => document.getElementById(id);

function showLesson() {
  const track = location.hash.slice(1);
  const lesson = lessons[track];
  element("lesson").hidden = !lesson;
  document.querySelectorAll("[data-track]").forEach((link) => {
    if (link.dataset.track === track) link.setAttribute("aria-current", "true");
    else link.removeAttribute("aria-current");
  });
  if (!lesson) return;
  activeTrack = track;
  element("lesson-meta").textContent = `${lesson.label} / Introduction · About ${lesson.minutes} minutes`;
  element("completion").textContent = completed.has(track) ? "Completed this visit" : "Sample lesson";
  element("lesson-title").textContent = lesson.title;
  element("explanation").replaceChildren(...lesson.paragraphs.map((text) => {
    const paragraph = document.createElement("p");
    paragraph.textContent = text;
    return paragraph;
  }));
  element("question").textContent = lesson.question;
  element("answers").replaceChildren(...lesson.answers.map(([text], index) => {
    const label = document.createElement("label");
    label.className = "answer";
    const input = document.createElement("input");
    input.type = "radio";
    input.name = "answer";
    input.value = index;
    input.required = true;
    label.append(input, document.createTextNode(text));
    return label;
  }));
  element("references").replaceChildren(...lesson.references.map(([title, href]) => {
    const item = document.createElement("li");
    const link = document.createElement("a");
    link.textContent = title;
    link.href = href;
    link.target = "_blank";
    link.rel = "noopener";
    item.append(link);
    return item;
  }));
  element("feedback").textContent = "";
  delete element("feedback").dataset.correct;
  element("lesson-title").focus();
}

element("quiz").addEventListener("submit", (event) => {
  event.preventDefault();
  const selected = new FormData(event.currentTarget).get("answer");
  if (selected === null) return;
  const [, correct, explanation] = lessons[activeTrack].answers[Number(selected)];
  element("feedback").dataset.correct = String(correct);
  element("feedback").textContent = `${explanation} ${correct ? "You can try another track or review the sources." : "Review the explanation or sources, then choose another answer and check again."}`;
  if (correct) {
    completed.add(activeTrack);
    element("completion").textContent = "Completed this visit";
    document.querySelector(`[data-track="${activeTrack}"] .track-status`).textContent = "Sample lesson completed ✓";
  }
});

window.addEventListener("hashchange", showLesson);
showLesson();
