import { describe, expect, it } from "vitest";
import { MockPlatform } from "./MockPlatform";
import { additionalFeedUrl, captureDate } from "./fixtures";

const signal = () => new AbortController().signal;

describe("capture platform", () => {
  it("keeps collection counts and committed events consistent after edits", async () => {
    const platform = new MockPlatform();
    const events: string[] = [];
    const stop = await platform.onUnreadChanged((entry) =>
      events.push(entry.id)
    );
    await platform.readArticle(signal(), "garden");
    expect(events).toEqual(["garden"]);
    expect(await platform.getAllEntitiesCounts()).toEqual({
      entryCount: 3,
      unreadCount: 1,
    });
    expect((await platform.listFeeds(signal()))[0].unreadCount).toBe(1);
    expect(
      (await platform.readTopic(signal(), "observations")).unreadCount
    ).toBe(0);
    expect(
      (await platform.listSites(signal())).find(
        (s) => s.id === "fieldnotes.example"
      )?.unreadCount
    ).toBe(1);
    expect(await platform.countTagUnread(signal(), ["outdoors"])).toBe(0);
    stop();
    await platform.markEntryUnread(signal(), "garden");
    expect(events).toHaveLength(1);
  });

  it("returns detached data and projects pin membership from its central store", async () => {
    const platform = new MockPlatform();
    const article = await platform.readArticleScope(signal(), "garden");
    article.title = "Caller edit";
    expect((await platform.readEntry(signal(), "garden")).title).toBe(
      "A garden for every season"
    );
    await platform.setPin({ type: "entry", id: "garden" }, true);
    expect(
      (await platform.listPinnedEntries(signal())).map((a) => a.id)
    ).toEqual(["garden"]);
    expect((await platform.readEntry(signal(), "garden")).isPinned).toBe(true);
    await platform.setPin({ type: "entry", id: "garden" }, false);
    expect(await platform.listPinnedEntries(signal())).toEqual([]);
  });

  it("replays generated identities, dates, and edits from a fresh instance", async () => {
    async function replay() {
      const platform = new MockPlatform();
      const feed = await platform.addFeed(signal(), additionalFeedUrl);
      const topic = await platform.createTopic(signal(), {
        title: "Reading projects",
      });
      const tag = await platform.createTag("Ideas");
      await platform.setEntryTag("garden", tag.id, true);
      await platform.addTopicEntity(
        signal(),
        topic.id,
        await platform.readEntry(signal(), "garden")
      );
      await platform.addBookmark(signal(), "garden", "blue");
      await platform.addAnnotation(signal(), {
        id: "new-annotation",
        reference: "garden",
        title: "Small changes",
        text: ["Small changes"],
        occurrence: [0],
        color: "blue",
        content: "Remember this",
      });
      return {
        feed,
        topic: await platform.readTopic(signal(), topic.id),
        tag,
        entries: await platform.listTagEntities(signal(), [tag.id]),
        saved: await platform.listAllEntities(signal(), "saved"),
      };
    }
    expect(await replay()).toEqual(await replay());
    const platform = new MockPlatform();
    expect(await platform.getAllEntitiesCounts()).toEqual({
      entryCount: 3,
      unreadCount: 2,
    });
    expect(
      (await platform.readArticleScope(signal(), "garden")).createdAt
    ).toBe(captureDate);
    expect(await platform.listFeeds(signal())).toHaveLength(1);
  });

  it("updates saved annotations and tag assignments through replacement and merge", async () => {
    const platform = new MockPlatform();
    const annotation = (await platform.readArticleScope(signal(), "wood"))
      .annotations[0];
    await platform.addTopicEntity(signal(), "observations", annotation);
    await platform.replaceAnnotation(signal(), annotation.id, {
      ...annotation,
      id: "recolored",
      color: "blue",
    });
    expect(
      (
        await platform.listTopicEntities(
          signal(),
          "observations",
          "saved",
          "timestamp"
        )
      ).map((a) => a.id)
    ).toContain("recolored");
    expect((await platform.listAnnotations(signal())).map((a) => a.id)).toEqual(
      ["recolored"]
    );
    await platform.removeAnnotation(signal(), {
      id: "recolored",
      reference: "wood",
    });
    expect(await platform.listAnnotations(signal())).toEqual([]);
    expect(
      await platform.listTopicEntities(
        signal(),
        "observations",
        "saved",
        "timestamp"
      )
    ).toEqual([]);
    await platform.mergeTags("outdoors", "projects");
    expect(await platform.readEntryTags(signal(), "garden")).toEqual([
      "projects",
    ]);
    expect((await platform.listTags(signal())).map((t) => t.id)).toEqual([
      "projects",
    ]);
    await platform.deleteTag("projects");
    expect(await platform.readEntryTags(signal(), "garden")).toEqual([]);
    await platform.removeBookmark(signal(), "wood");
    expect(await platform.listAllEntities(signal(), "saved")).toEqual([]);
  });

  it("rejects native and arbitrary network operations instead of reporting success", async () => {
    const platform = new MockPlatform();
    await expect(platform.openFileDialog(true)).rejects.toMatchObject({
      code: "app:error",
    });
    await expect(platform.openUrl("https://example.com")).rejects.toMatchObject(
      { code: "app:error" }
    );
    await expect(
      platform.addFeed(signal(), "https://example.com/feed")
    ).rejects.toMatchObject({ code: "app:error" });
    const controller = new AbortController();
    controller.abort();
    await expect(
      platform.addFeed(controller.signal, additionalFeedUrl)
    ).rejects.toMatchObject({ name: "AbortError" });
  });
});
