import { Language } from "@mui/icons-material";
import { useCallback, useMemo } from "react";
import { useSidebarSort } from "../../hooks/useSidebarSort";
import useRouter from "../../hooks/useRouter";
import type { Site } from "../../types";
import sortSidebarCollections from "../../utils/sortSidebarCollections";
import ExpandableList from "./ExpandableList";
import css from "./SiteList.module.css";
import SidebarEntity from "./SidebarEntity";
import UnreadCount from "./UnreadCount";

type SiteListProps = {
  isExpanded?: boolean;
  sites: Site[];
  onClickHeader: () => void;
};

export default function SiteList({
  isExpanded = false,
  sites,
  onClickHeader,
}: SiteListProps) {
  const { selectSite, state } = useRouter();
  const { siteHostname } = state;
  const [sortMode] = useSidebarSort();
  const sortedSites = useMemo(
    () => sortSidebarCollections(sites, sortMode),
    [sites, sortMode]
  );
  const handleSelectSite = useCallback(
    (site: Site) => selectSite(site),
    [selectSite]
  );

  return (
    <ExpandableList
      section="sites"
      className={css.SiteList}
      count={sites.length}
      label="Sites"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
    >
      {sortedSites.map((site) => (
        <SidebarEntity
          collection={site}
          key={site.id}
          icon={Language}
          selected={siteHostname === site.id}
          onSelectCollection={handleSelectSite}
          renderSecondaryAction={() => <UnreadCount count={site.unreadCount} />}
        />
      ))}
    </ExpandableList>
  );
}
