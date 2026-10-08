import { lazy, type ReactNode } from "react";
import { isModuleResourceLoadError, loadForUpdate } from "./update-required.ts";

/** A missing lazy resource renders nothing; ordinary module errors keep their error handling. */
export function lazyForUpdate<Props>(
  load: () => Promise<{ readonly default: (props: Props) => ReactNode }>,
) {
  return lazy(async (): Promise<{ default: (props: Props) => ReactNode }> => {
    try {
      return await loadForUpdate(load);
    } catch (error) {
      if (!isModuleResourceLoadError(error)) {
        throw error;
      }

      const Empty = (_props: Props) => null;

      return { default: Empty };
    }
  });
}
