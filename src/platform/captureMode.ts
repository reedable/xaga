// A separate Vite mode is required; opening the native app in development does
// not opt into fixtures. Production builds cannot select the capture platform.
export const isCaptureMode =
  import.meta.env.DEV && import.meta.env.MODE === "capture";
