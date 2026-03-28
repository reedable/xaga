import type Logger from "./Logger";

export type Opts = Record<string, unknown> | undefined;
export type Awaitable<T> = T | Promise<T>;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type Fn<T extends unknown[] = any[], R = any> = (
  ...args: T
) => Awaitable<R>;

export type Decorator<O extends Opts = undefined> = <F extends Fn>(
  fn: F,
  opts?: O,
) => F;

export type DecoratedFn<F extends Fn> = F & {
  with: (<O extends Opts>(decorator: Decorator<O>, opts: O) => DecoratedFn<F>) &
    ((decorator: Decorator) => DecoratedFn<F>);
};

export function decorate<F extends Fn>(fn: F): DecoratedFn<F> {
  function _with<O extends Opts>(decorator: Decorator<O>, opts: O) {
    return decorate(decorator(fn, opts));
  }

  return Object.assign(fn, { with: _with }) as DecoratedFn<F>;
}

export const abortable: Decorator<{ signal: AbortSignal }> = (fn, opts) => {
  return async function (
    this: ThisParameterType<typeof fn>,
    ...args: unknown[]
  ) {
    if (opts?.signal.aborted) throw opts.signal.reason;
    let onAbort: (() => void) | null = null;

    const abortPromise = new Promise<never>((_, reject) => {
      onAbort = () => reject(opts?.signal.reason);
      opts?.signal.addEventListener("abort", onAbort, { once: true });
    });

    try {
      return await Promise.race([fn.apply(this, args), abortPromise]);
    } finally {
      if (onAbort) opts?.signal.removeEventListener("abort", onAbort);
    }
  } as typeof fn;
};

export const DEFAULT_WAIT = 50;

export const debounced: Decorator<{ wait: number }> = (fn, opts) => {
  let timer: ReturnType<typeof setTimeout> | null = null;

  return function (this: ThisParameterType<typeof fn>, ...args: unknown[]) {
    if (timer !== null) clearTimeout(timer);

    timer = setTimeout(() => {
      timer = null;
      fn.apply(this, args);
    }, opts?.wait ?? DEFAULT_WAIT);
  } as typeof fn;
};

export const retriable: Decorator<{
  attempts: number;
  delay?: number;
  logger?: Logger;
}> = (fn, opts) => {
  return async function (
    this: ThisParameterType<typeof fn>,
    ...args: unknown[]
  ) {
    const maxAttempts = opts?.attempts ?? 1;
    let lastError: unknown;

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      try {
        return await fn.apply(this, args);
      } catch (e) {
        lastError = e;

        if (attempt < maxAttempts - 1 && opts?.delay) {
          opts?.logger?.debug("waiting...", e);
          await new Promise((resolve) => setTimeout(resolve, opts.delay));
        }
      }
    }

    throw lastError;
  } as typeof fn;
};

export const logged: Decorator<{ logger: Logger; name: string }> = (
  fn,
  opts,
) => {
  return async function (
    this: ThisParameterType<typeof fn>,
    ...args: unknown[]
  ) {
    const logger = opts?.logger.entering(opts?.name, ...args);
    let result: unknown;

    try {
      result = await fn.apply(this, args);
      return result;
    } catch (e) {
      logger?.warn(e);
      result = e;
      throw e; // Caller must handle the error
    } finally {
      logger?.exiting(result);
    }
  } as typeof fn;
};
