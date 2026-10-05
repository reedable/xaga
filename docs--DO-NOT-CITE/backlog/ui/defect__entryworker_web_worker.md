# Move entryWorker back into a Web Worker

`src/text/entryWorker.ts` runs on the main thread; its own header comment records why: a `ReferenceError: Can't find variable: document` from the remark-stringify bundle chunk inside the worker context (some package in the unified/rehype/remark pipeline accesses the global `document`). Move it back into a Web Worker once that is resolved.
