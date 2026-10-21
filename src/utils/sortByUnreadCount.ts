type CountSummary = { title: string; unreadCount: number };

export default function sortByUnreadCount(a: CountSummary, b: CountSummary) {
  return b.unreadCount - a.unreadCount || a.title.localeCompare(b.title);
}
