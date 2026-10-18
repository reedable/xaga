type TopicItemAdded = { topicId: string; itemId: string };
type Listener = (event: TopicItemAdded) => void;

const listeners = new Set<Listener>();

export function emitTopicItemAdded(event: TopicItemAdded): void {
  listeners.forEach((listener) => listener(event));
}

export function onTopicItemAdded(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
