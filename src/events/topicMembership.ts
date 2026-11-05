type TopicEntityAdded = { topicId: string; entityId: string };
type Listener = (event: TopicEntityAdded) => void;

const listeners = new Set<Listener>();

export function emitTopicEntityAdded(event: TopicEntityAdded): void {
  listeners.forEach((listener) => listener(event));
}

export function onTopicEntityAdded(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
