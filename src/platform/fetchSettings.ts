export interface FetchSettings {
  refererAllowed: boolean;
  gzipAllowed: boolean;
  httpAllowed: boolean;
  connectNonstandardPortsAllowed: boolean;
  maxRedirects: string;
  loaderTimeoutSecs: string;
  feedTimeoutSecs: string;
  feedMaxBodyBytes: string;
}

export type FetchSettingChange = {
  [K in keyof FetchSettings]: { setting: K; value: FetchSettings[K] };
}[keyof FetchSettings];
