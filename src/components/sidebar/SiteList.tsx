import { Language } from "@mui/icons-material";
import { useCallback, useMemo } from "react";
import { useSidebarSort } from "../../hooks/useSidebarSort";
import useRouter from "../../hooks/useRouter";
import type { Site } from "../../types";
import sortSidebarCollections from "../../utils/sortSidebarCollections";
import ExpandableList from "./ExpandableList";
import css from "./SiteList.module.css";
import SidebarItem from "./SidebarItem";
import SidebarSortControl from "./SidebarSortControl";
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
  const [sortMode, setSortMode] = useSidebarSort("sites");
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
      className={css.SiteList}
      label="Sites"
      isExpanded={isExpanded}
      onClickHeader={onClickHeader}
    >
      {isExpanded && (
        <SidebarSortControl
          label="Sites"
          sortMode={sortMode}
          onChange={setSortMode}
        />
      )}

      {sortedSites.map((site) => (
        <SidebarItem
          collection={site}
          key={site.hostname}
          icon={Language}
          selected={siteHostname === site.hostname}
          onSelectCollection={handleSelectSite}
          renderSecondaryAction={() => <UnreadCount count={site.unreadCount} />}
        />
      ))}
    </ExpandableList>
  );
}
