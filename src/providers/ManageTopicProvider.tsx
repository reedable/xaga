import {
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import EditTopicDialog from "../components/dialogs/EditTopicDialog";
import useAbortController from "../hooks/useAbortController";
import { useCollections } from "../hooks/useCollections";
import { useEvents } from "../hooks/useEvents";
import { useLocalStorage } from "../hooks/useLocalStorage";
import { useTopics } from "../hooks/useTopics";
import { useNavigate } from "react-router-dom";
import type { Topic } from "../types";
import ManageTopicContext, {
  type ManageTopicContextType,
} from "./ManageTopicContext";

const sortByUnreadCount = (a: Topic, b: Topic) =>
  b.unreadCount - a.unreadCount || a.title.localeCompare(b.title);

interface ManageTopicProviderProps {
  children: ReactNode;
}

export function ManageTopicProvider({ children }: ManageTopicProviderProps) {
  const navigate = useNavigate();
  const [abort, abortControllerRef] = useAbortController();
  const { listTopics } = useCollections(abortControllerRef);
  const { saveTopic, createTopic, deleteTopic } = useTopics(abortControllerRef);
  const { onUnreadChanged, onEmbeddingsChanged } = useEvents();

  const [topics, setTopics] = useState<Topic[]>([]);
  const [editingTopic, setEditingTopic] = useState<Topic | null>(null);
  const [isAddingTopic, setIsAddingTopic] = useState(false);
  const [pinnedTopicIds, setPinnedTopicIds] = useLocalStorage<string[]>(
    "sidebar.pinnedTopics",
    []
  );

  const requestTopics = useCallback(async () => {
    const results = await listTopics(sortByUnreadCount);
    if (results) setTopics(results);
  }, [listTopics]);

  const handleSaveTopic = useCallback(
    async (
      title: string,
      content: string,
      threshold: number,
      pinned: boolean
    ) => {
      let topic: Topic | undefined;
      if (editingTopic) {
        topic = await saveTopic({ ...editingTopic, title, content, threshold });
      } else {
        topic = await createTopic({ title, content, threshold });
      }
      if (topic) {
        setPinnedTopicIds((prev) =>
          pinned
            ? prev.includes(topic.id)
              ? prev
              : [...prev, topic.id]
            : prev.filter((id) => id !== topic.id)
        );
        await requestTopics();
        navigate(`/${topic.id}?filter=unread`);
      }
    },
    [editingTopic, saveTopic, createTopic, setPinnedTopicIds, requestTopics, navigate]
  );

  const handleDeleteTopic = useCallback(
    async (topic: Topic) => {
      await deleteTopic(topic.id);
      setPinnedTopicIds((prev) => prev.filter((id) => id !== topic.id));
      await requestTopics();
    },
    [deleteTopic, setPinnedTopicIds, requestTopics]
  );

  useEffect(
    () => onUnreadChanged(() => requestTopics()),
    [onUnreadChanged, requestTopics]
  );
  useEffect(
    () => onEmbeddingsChanged(() => requestTopics()),
    [onEmbeddingsChanged, requestTopics]
  );

  useEffect(() => {
    abort();
    requestTopics();
  }, [abort, requestTopics]);

  const value = useMemo<ManageTopicContextType>(
    () => ({
      topics,
      pinnedTopicIds,
      startEditingTopic: setEditingTopic,
      startAddingTopic: () => setIsAddingTopic(true),
    }),
    [topics, pinnedTopicIds]
  );

  return (
    <ManageTopicContext.Provider value={value}>
      {children}
      <EditTopicDialog
        open={editingTopic !== null || isAddingTopic}
        topic={editingTopic}
        pinned={pinnedTopicIds.includes(editingTopic?.id ?? "")}
        onClose={() => {
          setEditingTopic(null);
          setIsAddingTopic(false);
        }}
        onSave={handleSaveTopic}
        onDelete={handleDeleteTopic}
      />
    </ManageTopicContext.Provider>
  );
}

export function useManageTopic() {
  const ctx = useContext(ManageTopicContext);
  if (!ctx)
    throw new Error("useManageTopic must be used inside ManageTopicProvider");
  return ctx;
}
