import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { JSDOM } from 'jsdom';
const dir = path.dirname(fileURLToPath(import.meta.url));
const read = (file) => fs.readFileSync(path.join(dir, file), 'utf8');
const key = 'xaga-training-v1';
function page({url = 'http://localhost/docs/training/', stored, storedAnswers, fetch} = {}) {
  const dom = new JSDOM(read('index.html'), {url, runScripts: 'outside-only'});
  const w = dom.window;
  w.HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  w.HTMLDialogElement.prototype.close = function () { this.open = false; this.dispatchEvent(new w.Event('close')); };
  if (stored !== undefined) w.localStorage.setItem(key, stored);
  if (storedAnswers !== undefined) w.localStorage.setItem("xaga-training-answers-v1", storedAnswers);
  w.fetch = fetch || (() => Promise.reject(new Error('No fixture')));
  w.eval(`${read('lessons.js')}\n${read('training.js')}\nwindow.curriculum = TRAINING;`);
  return {w, tracks: w.curriculum, get: (id) => w.document.getElementById(id), close: () => w.close()};
}
function route(p, hash) { p.w.history.replaceState(null, '', `#${hash}`); p.w.dispatchEvent(new p.w.HashChangeEvent('hashchange')); }
const plainText = (text) => text.replace(/`([^`]+)`/g, "$1");
function choose(p, form, text) {
  const label = [...form.querySelectorAll('label')].find((node) => node.querySelector('.answer-text').textContent === plainText(text));
  assert.ok(label, text);
  label.querySelector('input').checked = true;
  label.querySelector('input').dispatchEvent(new p.w.Event('change', {bubbles: true}));
  return form.querySelector('.feedback');
}
const formFeedbackCount = (form) => form.querySelectorAll('.feedback').length;
const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

test('three progressive tracks, complete questions, valid references and Markdown headings', () => {
  const p = page();
  try {
    assert.equal(Object.keys(p.tracks).length, 3);
    for (const track of Object.values(p.tracks)) {
      assert.equal(track.lessons.length, 10);
      assert.equal(new Set(track.lessons.map(l => l.id)).size, 10);
      assert.deepEqual(Array.from(track.lessons.map(l => l.level)), [...Array(3).fill('Introduction'), ...Array(4).fill('Applied'), ...Array(3).fill('Advanced')]);
      for (const lesson of track.lessons) {
        assert.ok(lesson.objective && lesson.example && lesson.paragraphs.length >= 3);
        assert.equal(lesson.questions.length, 2);
        for (const question of lesson.questions) {
          assert.equal(question.answers.length, 3);
          assert.equal(question.answers.filter(a => a.correct).length, 1);
          question.answers.forEach(a => assert.ok(a.text && a.feedback));
        }
        for (const ref of lesson.references) {
          const [relative, anchor] = ref.path.split('#');
          const file = path.resolve(dir, relative);
          assert.ok(fs.existsSync(file), `${lesson.id}: ${relative}`);
          if (anchor && file.endsWith('.md')) {
            const headings = fs.readFileSync(file, 'utf8').split('\n').filter(l => /^#+ /.test(l)).map(l => l.replace(/^#+ /, '').toLowerCase().replace(/[^\p{L}\p{N}_\-\s]/gu, '').replace(/ /g, '-'));
            assert.ok(headings.includes(anchor), ref.path);
          }
        }
      }
    }
  } finally { p.close(); }
});

test('all 180 answer paths explain the answer and all 30 lessons can complete', () => {
  const p = page();
  try {
    for (const [id, track] of Object.entries(p.tracks)) {
      for (const lesson of track.lessons) {
        route(p, `${id}/${lesson.id}`);
        assert.equal(p.get('lesson-title').textContent, lesson.title);
        assert.equal(p.get('lesson-meta'), null);
        assert.equal(p.get('curriculum').querySelector('.track-duration').textContent, 'About 50–80 minutes total');
        assert.equal(p.get('curriculum').querySelectorAll('.lesson-duration').length, 10);
        assert.equal(p.w.document.querySelectorAll('#curriculum [aria-current="step"]').length, 1);
        const forms = [...p.get('quizzes').querySelectorAll('form')];
        lesson.questions.forEach((question, i) => question.answers.forEach(answer => {
          const result = choose(p, forms[i], answer.text);
          assert.equal(result.dataset.correct, String(answer.correct));
          assert.ok(result.closest('.answer').querySelector('input').checked);
          assert.equal(formFeedbackCount(forms[i]), 1);
          assert.equal(result.closest('.answer').querySelector('.answer-indicator').textContent, answer.correct ? '✓' : '✕');
          assert.equal(result.closest('.answer').querySelector('.answer-indicator').getAttribute('aria-hidden'), 'true');
          assert.ok(result.textContent.includes(plainText(answer.feedback)));
        }));
        assert.equal(p.w.document.querySelector('#curriculum [aria-current=step] .lesson-status').textContent, 'Completed');
      }
      assert.equal(p.get('track-progress').textContent, '10 / 10 lessons completed');
    }
  } finally { p.close(); }
});

test('selection immediately updates feedback without a submit button', () => {
  const p = page();
  try {
    route(p, 'product/purpose');
    const form = p.get('quizzes').querySelector('form');
    choose(p, form, p.tracks.product.lessons[0].questions[0].answers[1].text);
    assert.equal(p.w.document.querySelector('#curriculum [aria-current=step] .lesson-status').textContent, '');
    assert.equal(form.querySelector('button[type=submit]'), null);
    const correct = p.tracks.product.lessons[0].questions[0].answers[0];
    const feedback = choose(p, form, correct.text);
    assert.ok(feedback.textContent.includes(plainText(correct.feedback)));
    assert.equal(feedback.dataset.correct, 'true');
    assert.equal(p.w.document.querySelectorAll('.feedback[data-correct=true]').length, 1);
  } finally { p.close(); }
});

test('saved progress survives reload and reset leaves unrelated storage alone', () => {
  const p = page(); let saved;
  try {
    route(p, 'product/purpose');
    [...p.get('quizzes').querySelectorAll('form')].forEach((form, i) => choose(p, form, p.tracks.product.lessons[0].questions[i].answers[0].text));
    saved = p.w.localStorage.getItem(key);
  } finally { p.close(); }
  const r = page({url: 'http://localhost/docs/training/#product/purpose', stored: saved});
  try {
    assert.equal(r.w.document.querySelector('#curriculum [aria-current=step] .lesson-status').textContent, 'Completed');
    r.w.localStorage.setItem('unrelated', 'keep');
    r.get('reset-progress').click();
    assert.equal(r.w.document.querySelector('#curriculum [aria-current=step] .lesson-status').textContent, '');
    assert.equal(r.w.localStorage.getItem('unrelated'), 'keep');
    assert.equal(r.w.localStorage.getItem(key), '{}');
  } finally { r.close(); }
});

test('invalid storage and routes are handled safely', () => {
  for (const stored of ['bad JSON', 'null', '[]', '{"product/purpose":[0,0,1,999,-1,"0"]}']) {
    const p = page({stored});
    try {
      route(p, 'product/purpose');
      assert.equal(p.get('course').hidden, false);
      assert.equal(p.get('completion'), null);
      for (const hash of ['missing', '__proto__', 'constructor', 'product/missing']) {
        route(p, hash);
        assert.equal(p.get('course').hidden, true);
        assert.ok(p.get('route-message').textContent);
      }
      route(p, 'engineering'); assert.equal(p.get('course').hidden, false);
      route(p, ''); assert.equal(p.get('course').hidden, true);
    } finally { p.close(); }
  }
});

test('source viewer uses safe text and returns focus to its opener', async () => {
  const p = page({fetch: async () => ({ok: true, text: async () => '<img src=x onerror="window.injected=true">'})});
  try {
    route(p, 'product/purpose');
    const opener = p.get('references').querySelector('button'); opener.click(); await tick();
    assert.equal(p.get('source-dialog').open, true);
    assert.equal(p.get('source-content').querySelector('img'), null);
    assert.ok(p.get('source-content').textContent.includes('<img'));
    assert.equal(p.w.injected, undefined);
    p.get('close-source').click();
    assert.equal(p.w.document.activeElement, opener);
  } finally { p.close(); }
});

test('source fetch failures retain a usable original link', async () => {
  const p = page();
  try {
    route(p, 'product/purpose'); p.get('references').querySelector('button').click(); await tick();
    assert.ok(p.get('source-status').textContent.includes('unavailable'));
    assert.ok(p.get('source-original').href.endsWith('/README.md'));
  } finally { p.close(); }
});

test('cancelled source requests cannot replace a newer preview', async () => {
  let finishOld; let calls = 0;
  const p = page({fetch: () => ++calls === 1 ? new Promise(resolve => { finishOld = resolve; }) : Promise.resolve({ok: true, text: async () => 'new source'})});
  try {
    route(p, 'product/purpose');
    const openers = p.get('references').querySelectorAll('button');
    openers[0].click(); p.get('close-source').click(); openers[1].click(); await tick();
    finishOld({ok: true, text: async () => 'stale source'}); await tick();
    assert.equal(p.get('source-content').textContent, 'new source');
  } finally { p.close(); }
});

test('file opening supports lessons without storage or source fetch', () => {
  const p = page({url: `file://${path.join(dir, 'index.html')}#interface/motion`});
  try {
    assert.equal(p.get('course').hidden, false);
    assert.ok(p.get('storage-note').textContent.includes('this visit'));
    p.get('references').querySelector('button').click();
    assert.ok(p.get('source-status').textContent.includes('Open source in a new tab'));
    p.get('close-source').click();
    const lesson = p.tracks.interface.lessons.find(l => l.id === 'motion');
    choose(p, p.get('quizzes').querySelector('form'), lesson.questions[0].answers[0].text);
    assert.equal(p.w.document.querySelectorAll('.feedback[data-correct=true]').length, 1);
  } finally { p.close(); }
});

test('correct and incorrect selections and feedback survive navigation, reload, and reset', () => {
  const p = page();
  let stored, storedAnswers;
  const verify = (current) => {
    const forms = current.get('quizzes').querySelectorAll('form');
    assert.equal(forms[0].querySelector('.answer-indicator:not(:empty)').textContent, '✓');
    assert.equal(forms[1].querySelector('.answer-indicator:not(:empty)').textContent, '✕');
    assert.equal(forms[0].querySelector('.feedback').dataset.correct, 'true');
    assert.equal(forms[1].querySelector('.feedback').dataset.correct, 'false');
    for (const form of forms) assert.ok(form.querySelector('.feedback').closest('.answer').querySelector('input').checked);
  };
  try {
    route(p, 'product/purpose');
    const lesson = p.tracks.product.lessons[0];
    const forms = p.get('quizzes').querySelectorAll('form');
    choose(p, forms[0], lesson.questions[0].answers[0].text);
    choose(p, forms[1], lesson.questions[1].answers[1].text);
    route(p, 'engineering/layers');
    route(p, 'product/purpose');
    verify(p);
    stored = p.w.localStorage.getItem(key);
    storedAnswers = p.w.localStorage.getItem('xaga-training-answers-v1');
  } finally { p.close(); }
  const restored = page({url: 'http://localhost/docs/training/#product/purpose', stored, storedAnswers});
  try {
    verify(restored);
    restored.get('reset-progress').click();
    assert.equal(restored.get('quizzes').querySelectorAll('input:checked').length, 0);
    assert.equal(restored.get('quizzes').querySelectorAll('.feedback:not(:empty)').length, 0);
    assert.equal(restored.w.localStorage.getItem('xaga-training-answers-v1'), '{}');
  } finally { restored.close(); }
});

test('answers persist during navigation when file storage is unavailable', () => {
  const p = page({url: `file://${path.join(dir, 'index.html')}#interface/motion`});
  try {
    const lesson = p.tracks.interface.lessons.find(l => l.id === 'motion');
    choose(p, p.get('quizzes').querySelector('form'), lesson.questions[0].answers[1].text);
    p.w.location.hash = 'interface/port';
    p.w.dispatchEvent(new p.w.HashChangeEvent('hashchange'));
    p.w.location.hash = 'interface/motion';
    p.w.dispatchEvent(new p.w.HashChangeEvent('hashchange'));
    const form = p.get('quizzes').querySelector('form');
    assert.ok(form.querySelector('input:checked'));
    assert.equal(form.querySelector('.feedback').dataset.correct, 'false');
  } finally { p.close(); }
});

test('inline identifiers render as code without interpreting HTML', () => {
  const p = page();
  try {
    route(p, 'interface/port');
    assert.ok([...p.get('explanation').querySelectorAll('code')].some(node => node.textContent === 'Platform'));
    assert.equal(p.get('explanation').textContent.includes('`'), false);
    const lesson = p.tracks.interface.lessons[0];
    choose(p, p.get('quizzes').querySelector('form'), lesson.questions[0].answers[0].text);
    assert.equal(p.get('quizzes').querySelector('.answer-text code').textContent, 'Platform');
    lesson.paragraphs[0] = 'Use `<script>unsafe()</script>` and <img src=x onerror=unsafe()>.';
    route(p, 'interface/port');
    assert.equal(p.get('explanation').querySelector('script, img'), null);
    assert.equal(p.get('explanation').querySelector('code').textContent, '<script>unsafe()</script>');
  } finally { p.close(); }
});
