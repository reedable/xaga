import {
  useEffect,
  useState,
  useCallback,
  useMemo,
  useRef,
  type ReactNode,
} from "react";
import { usePlatform } from "../platform/Platform";
import { useErrorSink } from "../hooks/useErrorSink";
import { pinTargets, type PinTarget, type PinNode, type PinEdit } from "../pins";
import { PinsContext } from "./PinsContext";

export default function PinsProvider({ children }: { children: ReactNode }) {
  const platform = usePlatform();
  const { pushError } = useErrorSink();
  const [tree, setPins] = useState<PinNode[]>([]);
  const pins = useMemo(() => pinTargets(tree), [tree]);
  const requests = useRef({ generation: 0, active: false });
  const controller = useRef(new AbortController());
  const refresh = useCallback(async () => {
    const request = ++requests.current.generation;
    const result = await platform.listPins(controller.current.signal);
    if (requests.current.active && request === requests.current.generation)
      setPins(result);
  }, [platform]);
  useEffect(() => {
    const state = requests.current;
    state.active = true;
    let active = true;
    let stop: (() => void) | undefined;
    void (async () => {
      stop = await platform.onPinChanged(() => {
        if (active) void refresh().catch(pushError);
      });
      if (!active) {
        stop();
        return;
      }
      await refresh();
    })().catch((error) => {
      if (active) pushError(error);
    });
    return () => {
      active = false;
      state.active = false;
      ++state.generation;
      stop?.();
    };
  }, [platform, refresh, pushError]);
  const setPin = useCallback(
    async (target: PinTarget, pinned: boolean) => {
      await platform.setPin(target, pinned);
      await refresh();
    },
    [platform, refresh]
  );
  const organizePins = useCallback(async (edit: PinEdit) => {
    await platform.organizePins(edit);
    await refresh();
  }, [platform, refresh]);
  const value = useMemo(
    () => ({ pins, tree, setPin, organizePins, refreshPins: refresh }),
    [pins, tree, setPin, organizePins, refresh]
  );
  return <PinsContext.Provider value={value}>{children}</PinsContext.Provider>;
}
