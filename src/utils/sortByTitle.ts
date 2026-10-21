type Titled = { title: string };

export default function sortByTitle(a: Titled, b: Titled) {
  return a.title.localeCompare(b.title);
}
