import { Typography } from "@mui/material";
import { useCallback, useEffect, useRef, useState } from "react";
import { useArticles } from "../../hooks/useArticles";
import {
  isAnnotation,
  isEntry,
  type Annotation,
  type Entry,
  type Item,
} from "../../types";
import cx from "../../utils/cx";
import ArticleReader from "./ArticleReader";
import css from "./_.module.css";

interface ItemViewProps {
  className?: string;
  selectedItem: Item | null; // FIXME Should this accept Item type?
  onSelectRelatedItem: (item: Item) => void;
  onChangeItem?: (item: Item) => void;
}

export function ItemView({
  className,
  selectedItem,
  onSelectRelatedItem,
  onChangeItem,
}: ItemViewProps) {
  const abortControllerRef = useRef(new AbortController());
  const { readArticle } = useArticles(abortControllerRef);
  const [referenceEntry, setReferenceEntry] = useState<Entry | null>(null);

  const requestReferenceEntry = useCallback(
    async (annotation: Annotation) => {
      const entry = await readArticle({
        id: annotation.reference,
      });

      if (entry) setReferenceEntry(entry);
    },
    [readArticle],
  );

  useEffect(() => {
    if (isAnnotation(selectedItem)) {
      (async () => requestReferenceEntry(selectedItem))();
    }
  }, [requestReferenceEntry, selectedItem]);

  if (!selectedItem) {
    return (
      <div className={cx(className, css.Placeholder)}>
        <Typography variant="body2" color="text.disabled">
          Select an article
        </Typography>
      </div>
    );
  }

  return (
    <div className={cx(className, css.LayoutContainer)}>
      <div className={css.Layout}>
        {isEntry(selectedItem) ? (
          <ArticleReader
            selectedEntry={selectedItem}
            onChangeEntry={onChangeItem}
            onSelectEntry={onSelectRelatedItem}
          />
        ) : isAnnotation(selectedItem) && referenceEntry ? (
          <ArticleReader
            selectedEntry={referenceEntry}
            onChangeEntry={onChangeItem}
            onSelectEntry={onSelectRelatedItem}
          />
        ) : null}
      </div>
    </div>
  );
}
