import { describe, expect, it } from "vitest";
import sortSidebarCollections from "./sortSidebarCollections";

const collections = [
  { title: "Zulu", unreadCount: 2 },
  { title: "Alpha", unreadCount: 2 },
  { title: "Bravo", unreadCount: 0 },
];

describe("sortSidebarCollections", () => {
  it("sorts by unread count descending with an alphabetical tie-break", () => {
    expect(
      sortSidebarCollections(collections, "unread").map(({ title }) => title)
    ).toEqual(["Alpha", "Zulu", "Bravo"]);
  });

  it("sorts alphabetically by title", () => {
    expect(
      sortSidebarCollections(collections, "alphabetical").map(
        ({ title }) => title
      )
    ).toEqual(["Alpha", "Bravo", "Zulu"]);
  });

  it("does not change provider-owned order", () => {
    sortSidebarCollections(collections, "alphabetical");

    expect(collections.map(({ title }) => title)).toEqual([
      "Zulu",
      "Alpha",
      "Bravo",
    ]);
  });
});
