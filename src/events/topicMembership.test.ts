import { expect, it, vi } from "vitest";
import { emitTopicItemAdded, onTopicItemAdded } from "./topicMembership";

it("notifies current subscribers only", () => {
  const listener = vi.fn();
  const unsubscribe = onTopicItemAdded(listener);
  const event = { topicId: "topic", itemId: "entry" };

  emitTopicItemAdded(event);
  unsubscribe();
  emitTopicItemAdded(event);

  expect(listener).toHaveBeenCalledOnce();
  expect(listener).toHaveBeenCalledWith(event);
});
