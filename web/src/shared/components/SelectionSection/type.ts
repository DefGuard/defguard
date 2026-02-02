import type { MouseEventHandler, ReactNode } from 'react';

export type SelectionKey = string | number;

export type SelectionSectionCustomItemRenderProps<T extends SelectionKey, M = unknown> = {
  active: boolean;
  option: SelectionOption<T, M>;
  onClick: MouseEventHandler<HTMLDivElement>;
};

export type SelectionSectionCustomRender<T extends SelectionKey, M = unknown> = (
  props: SelectionSectionCustomItemRenderProps<T, M>,
) => ReactNode;

export type SelectionOption<T, M = unknown> = {
  id: T;
  label: string;
  meta?: M;
  // if there is a need to search in more then label itself
  searchFields?: string[];
};

export interface SelectionSectionProps<T extends SelectionKey, M = unknown> {
  selection: Set<T>;
  onChange: (value: Set<T>) => void;
  options: SelectionOption<T, M>[];
  orderItems?: (items: SelectionOption<T, M>[]) => SelectionOption<T, M>[];
  renderItem?: SelectionSectionCustomRender<T, M>;
  enableDividers?: boolean;
  itemHeight?: number;
  itemGap?: number;
  className?: string;
  id?: string;
}
