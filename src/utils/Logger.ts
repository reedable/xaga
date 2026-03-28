//
// FIXME context tracking is broken
//
export default class Logger {
  static getLogger(name = "DEFAULT") {
    return new Logger(name);
  }

  private names: string[] = [];
  private trace: unknown[] | null = null;

  constructor(...names: string[]) {
    this.names = [...names];
  }

  error(...args: unknown[]) {
    if (this.trace) {
      console.info(`[${this.names.join(":")}]`, ...this.trace);
    }
    console.error(`[${this.names.join(":")}]`, ...args);
  }

  warn(...args: unknown[]) {
    this.trace = [...args];
    console.warn(`[${this.names.join(":")}]`, ...args);
  }

  log(...args: unknown[]) {
    this.trace = [...args];
    console.log(`[${this.names.join(":")}]`, ...args);
  }

  debug(...args: unknown[]) {
    this.trace = [...args];
    console.debug(`[${this.names.join(":")}]`, ...args);
  }

  entering(name: string, ...rest: unknown[]) {
    const nested = new Logger(...this.names, name);
    nested.debug("entering", ...rest);
    return nested;
  }

  exiting(result: unknown) {
    this.debug("exiting", result);
  }
}
