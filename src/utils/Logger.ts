export default class Logger {
  static getLogger(name: string = "Logger") {
    return new Logger(name);
  }

  private names: string[] = [];

  constructor(...names: string[]) {
    this.names = [...names];
  }

  getLogger(name: string) {
    return new Logger(...[...this.names, name]);
  }

  error(...args: unknown[]) {
    console.error(`[${this.names.join(":")}]`, ...args);
  }

  warn(...args: unknown[]) {
    console.warn(`[${this.names.join(":")}]`, ...args);
  }

  log(...args: unknown[]) {
    console.log(`[${this.names.join(":")}]`, ...args);
  }

  debug(...args: unknown[]) {
    console.debug(`[${this.names.join(":")}]`, ...args);
  }
}
