import { expect, it, vi } from "vitest";
import { emitTopicEntityAdded, onTopicEntityAdded } from "./topicMembership";

it("notifies current subscribers only", () => {
  const listener = vi.fn();
  const unsubscribe = onTopicEntityAdded(listener);
  const event = { topicId: "topic", entityId: "entry" };

  emitTopicEntityAdded(event);
  unsubscribe();
  emitTopicEntityAdded(event);

  expect(listener).toHaveBeenCalledOnce();
  expect(listener).toHaveBeenCalledWith(event);
});
