# Xaga training

Open [index.html](index.html) to choose a track. Each track has ten lessons:
three introductory, four applied, and three advanced. Each lesson includes an
objective, explanation, Xaga example, references, and two retryable questions
with answer-specific feedback. Lessons are freely navigable; correct answers
record progress rather than locking access to later material.

## Running locally

Opening `index.html` directly works without a build or Rust backend. Source
files can be opened in separate tabs, subject to browser file handling.

For the in-page source viewer, serve the **repository root** (`xaga/`) so that
both documentation and source paths are accessible:

```sh
cd /path/to/xaga
python3 -m http.server 8000 --bind 127.0.0.1
```

Visit <http://127.0.0.1:8000/docs/training/>. The viewer displays source as plain
text, preserving code and Markdown without executing their markup. Browser find
can locate headings or symbols. The training makes no external network requests
and has no analytics, CDN assets, or remote dependencies.

Completion is stored under `xaga-training-v1` and selected answers under
`xaga-training-answers-v1` in browser local storage, separately
for each origin. If storage is unavailable it lasts for the current visit.
Reset progress clears only the training's completion and selected answers.
Returning to a lesson restores the last selection and its feedback for each question. Completion means every
question in a lesson has been answered correctly at least once; retrying a
completed question does not revoke prior completion.

## Maintaining the material

`lessons.js` contains the curriculum; `training.js` handles navigation, quizzes,
progress, and source viewing. `styles.css` uses the system light/dark preference.
No build step or frontend framework is required.

Keep stable track and lesson IDs for bookmarks and saved progress. References
are relative to `index.html`. Link to authoritative docs and actual code paths;
identify target designs and known limits rather than describing every documented
intention as implemented. Scenarios illustrate documented behavior and are not screenshots
or claims of a browser-only mock of the application.

When changing a question's meaning, change its lesson ID or deliberately version
the saved-progress key so an older completion does not stand for a new exercise.
Keep one correct option per question and give every option an explanation.
Runtime display rotates option positions deterministically.

## Writing lessons

Use concrete actions and outcomes in explanations, questions, answers, and
feedback. Name the object being discussed: for example, introduce Research as
a tag name before asking what removing it does. Ask what the user or developer
would do or see, rather than asking an abstract question about a design.

Wrap inline code names, fields, and filenames in single backticks in lesson
content; the renderer creates semantic `<code>` elements without interpreting HTML.

Use established technical terms and actual code names when they help teach the
implementation. Explain them on first use in a lesson. Avoid invented labels
and metaphors that stand in for an explanation; say which component performs
an operation, which file saves the data, or which result changes. Product
lessons should describe the reader’s workflow before introducing storage types.

Wording-only changes can keep lesson IDs and answer order when the meaning and
correctness remain the same, preserving the trainee’s saved selections.

## Verification

From the repository root, with the existing development dependencies installed:

```sh
node --test docs/training/training.test.mjs
node --check docs/training/lessons.js
node --check docs/training/training.js
```

The checks cover curriculum integrity, local reference paths and Markdown
headings, all answer feedback, progress persistence and reset, route handling,
and source-viewer safety and failure behavior. They use jsdom; native browser
layout and platform-specific rendering still need visual review.
