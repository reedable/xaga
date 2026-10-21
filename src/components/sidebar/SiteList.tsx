import { Language } from "@mui/icons-material";
import { useCallback } from "react";
import useRouter from "../../hooks/useRouter";
import type { Site } from "../../types";
import ExpandableList from "./ExpandableList";
import css from "./SiteList.module.css";
import SidebarItem from "./SidebarItem";
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
      {sites.map((site) => (
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
