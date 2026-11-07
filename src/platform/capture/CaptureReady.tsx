import { useContext, useEffect } from "react";
import FeedsContext from "../../providers/FeedsContext";
import TopicsContext from "../../providers/TopicsContext";
import SitesContext from "../../providers/SitesContext";
import { TagsContext } from "../../providers/TagsContext";
import AllEntitiesCountsContext from "../../providers/AllEntitiesCountsContext";
import { useSettings } from "../../hooks/useSettings";

export function CaptureReady() {
  const feeds = useContext(FeedsContext);
  const topics = useContext(TopicsContext);
  const sites = useContext(SitesContext);
  const tags = useContext(TagsContext);
  const counts = useContext(AllEntitiesCountsContext);
  const { settingsLoaded } = useSettings();
  const ready =
    settingsLoaded &&
    feeds?.hasLoaded &&
    topics?.hasLoaded &&
    sites?.hasLoaded &&
    tags?.hasLoaded &&
    counts !== null;
  useEffect(() => {
    let active = true;
    if (ready)
      void document.fonts.ready.then(() => {
        requestAnimationFrame(() => {
          if (active) document.documentElement.dataset.captureReady = "true";
        });
      });
    return () => {
      active = false;
      delete document.documentElement.dataset.captureReady;
    };
  }, [ready]);
  return null;
}
