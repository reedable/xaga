import { useEffect, useState } from "react";
import { type Item } from "../types";
import formatByline from "../utils/formatByline";
import { DAY, getItemDate, HOUR, MINUTE } from "../utils/formatDate";

interface BylineProps {
  className?: string;
  item: Item;
}

function getDelay(ageMs: number) {
  if (ageMs < HOUR) return MINUTE - (ageMs % MINUTE);
  if (ageMs < DAY) return HOUR - (ageMs % HOUR);
  if (ageMs < 3 * DAY) return DAY - (ageMs % DAY);
  return null; // no tick needed
}

export function Byline({ className, item }: BylineProps) {
  const [, setTick] = useState(0);

  useEffect(() => {

    function schedule(): ReturnType<typeof setTimeout> | undefined {
      const date = getItemDate(item);
      if (!date) return;

      const ageMs = Date.now() - date.getTime();
      const delay = getDelay(ageMs);
      if (delay === null) return;

      return setTimeout(() => {
        setTick((t) => t + 1);
        schedule();
      }, delay);
    }

    const id = schedule();
    return () => clearTimeout(id!);
  }, [item]);

  return <div className={className}>{formatByline(item)}</div>;
}
