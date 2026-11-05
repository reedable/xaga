import { useEffect, useState } from "react";
import { type ManagedEntity } from "../types";
import cx from "../utils/cx";
import formatByline from "../utils/formatByline";
import { DAY, getEntityDate, HOUR, MINUTE } from "../utils/formatDate";
import css from "./Byline.module.css";

interface BylineProps {
  className?: string;
  entity: ManagedEntity;
}

function getDelay(ageMs: number) {
  if (ageMs < HOUR) return MINUTE - (ageMs % MINUTE);
  if (ageMs < DAY) return HOUR - (ageMs % HOUR);
  if (ageMs < 3 * DAY) return DAY - (ageMs % DAY);
  return null; // no tick needed
}

export function Byline({ className, entity }: BylineProps) {
  const [, setTick] = useState(0);

  useEffect(() => {
    function schedule(): ReturnType<typeof setTimeout> | undefined {
      const date = getEntityDate(entity);
      if (!date) return;

      const ageMs = Date.now() - date.getTime();
      const delay = getDelay(ageMs);
      if (delay === null) return;

      return setTimeout(() => {
        setTick((t) => t + 1);
        schedule();
      }, delay);
    }

    const tid = schedule();

    return () => {
      if (typeof tid === "number") clearTimeout(tid!);
    };
  }, [entity]);

  return (
    <span className={cx(className, css.Byline)}>{formatByline(entity)}</span>
  );
}
