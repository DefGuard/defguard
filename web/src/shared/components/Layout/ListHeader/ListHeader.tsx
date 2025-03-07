import './style.scss';

import clsx from 'clsx';

import { ListSortDirection } from '../../../defguard-ui/components/Layout/VirtualizedList/types';
import { isPresent } from '../../../defguard-ui/utils/isPresent';

type ListHeaderColumnProps<T> = {
  active: boolean;
  sortDirection?: ListSortDirection;
  onClick?: () => void;
} & ListHeaderColumnConfig<T>;

type ListHeaderColumnConfig<T> = {
  sortKey: keyof T;
  label: string;
  enabled?: boolean;
};

type Props<T> = {
  headers: ListHeaderColumnConfig<T>[];
  enabledSortKeys: Array<keyof T>;
  activeKey?: keyof T;
  sortDirection?: ListSortDirection;
  className?: string;
  id?: string;
  onChange?: (key: keyof T, direction: ListSortDirection) => void;
};

const ListHeaderColumn = <T extends object>({
  onClick,
  sortDirection,
  label,
  sortKey,
  active,
  enabled,
}: ListHeaderColumnProps<T>) => {
  const disabled = !enabled;
  return (
    <div
      className={clsx('list-header-column', {
        disabled: disabled,
        active: active && !disabled,
      })}
      data-testid={`list-header-${sortKey.toString()}`}
      data-direction={sortDirection?.valueOf().toLowerCase() ?? undefined}
    >
      <button type="button" onClick={onClick} disabled={disabled}>
        <p className="label">{label}</p>
        <div
          className={clsx('sort-icon', {
            desc: sortDirection === ListSortDirection.DESC,
            asc: sortDirection === ListSortDirection.ASC,
          })}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width={22}
            height={22}
            viewBox="0 0 22 22"
            fill="none"
          >
            <path
              d="M5.34282 9.75794L9.58546 14.0006C9.97599 14.3911 10.6091 14.3911 10.9997 14.0006C11.3902 13.6101 11.3902 12.9769 10.9997 12.5864L6.75703 8.34372C6.36651 7.9532 5.73334 7.9532 5.34282 8.34372C4.9523 8.73425 4.9523 9.36741 5.34282 9.75794Z"
              fill="#899CA8"
            />
            <path
              d="M11.3428 13.9994L15.5855 9.75679C15.976 9.36627 15.976 8.7331 15.5855 8.34258C15.1949 7.95205 14.5618 7.95205 14.1712 8.34258L9.92861 12.5852C9.53808 12.9757 9.53808 13.6089 9.92861 13.9994C10.3191 14.39 10.9523 14.39 11.3428 13.9994Z"
              fill="#899CA8"
            />
          </svg>
        </div>
      </button>
    </div>
  );
};

export const ListHeader = <T extends object>({
  headers,
  activeKey,
  sortDirection,
  className,
  id,
  onChange,
}: Props<T>) => {
  return (
    <div className={clsx('list-headers', className)} id={id}>
      {headers.map(({ label, sortKey, enabled }) => {
        const isActive = activeKey === sortKey;
        const direction = isActive ? sortDirection : ListSortDirection.ASC;
        return (
          <ListHeaderColumn
            key={sortKey as string}
            sortDirection={direction}
            active={isActive}
            label={label}
            sortKey={sortKey}
            onClick={() => {
              if (enabled && isPresent(onChange)) {
                if (isActive) {
                  const newDirection =
                    sortDirection === ListSortDirection.ASC
                      ? ListSortDirection.DESC
                      : ListSortDirection.ASC;
                  onChange(sortKey, newDirection);
                } else {
                  onChange(sortKey, ListSortDirection.ASC);
                }
              }
            }}
          />
        );
      })}
    </div>
  );
};
