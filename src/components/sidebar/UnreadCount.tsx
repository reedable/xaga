import css from "./UnreadCount.module.css";

type Props = { count: number };

export default function UnreadCount({ count }: Props) {
  if (count === 0) return null;
  return <span className={css.UnreadCount}>{count}</span>;
}
