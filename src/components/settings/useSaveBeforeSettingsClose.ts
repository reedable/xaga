import { useContext, useEffect, useEffectEvent } from "react";
import { SettingsSaveContext } from "./SettingsSaveContext";

export function useSaveBeforeSettingsClose(save: () => Promise<boolean>) {
  const saves = useContext(SettingsSaveContext);
  if (!saves) throw new Error("Numeric settings require SettingsSaveProvider");
  const saveLatest = useEffectEvent(save);
  useEffect(() => {
    const callback = () => saveLatest();
    saves.add(callback);
    return () => {
      saves.delete(callback);
    };
  }, [saves]);
}
