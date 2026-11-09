import { Typography } from "@mui/material";
import { FetchNumber, FetchSwitch } from "./FetchSettingControls";

export function SecuritySettings() {
  return (
    <div>
      <Typography component="h1" variant="h6" gutterBottom>
        Security
      </Typography>
      <FetchSwitch
        setting="httpAllowed"
        label="Allow unencrypted HTTP"
        description="Off by default. Allows HTTP requests routed through Xaga’s network proxy, including feed requests and article-loader requests that use the proxy. HTTP traffic can be read and changed in transit. Changes apply to new proxy requests; existing connections are unaffected. WebKit can bypass the proxy for some local requests, so this control does not enforce all webview traffic."
      />
      <FetchSwitch
        setting="connectNonstandardPortsAllowed"
        label="Allow HTTPS connections on other ports"
        description="Off by default. Xaga’s proxy permits CONNECT tunnels only to port 443. Enabling this permits tunnels to any port for feeds and article loads routed through the proxy, increasing the services a website can reach. Destination address checks still apply; the proxy cannot inspect traffic inside a tunnel. Changes apply to new tunnels, not existing connections. WebKit requests that bypass the proxy are not covered."
      />
      <FetchNumber
        setting="maxRedirects"
        label="Maximum feed redirects"
        maximum={10n}
        unit="hops"
        description="Defaults to 3. Zero refuses the first redirect. Raising the limit lets feeds follow more website-selected destinations. Applies to the next feed add, refresh, or import; does not limit redirects in article-loader windows."
      />
    </div>
  );
}
