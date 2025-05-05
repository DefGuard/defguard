import './style.scss';

import clsx from 'clsx';
import { flatten, orderBy } from 'lodash-es';
import { PropsWithChildren, useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ArrowSingleDirection } from '../../../defguard-ui/components/icons/ArrowSingle/types';
import { Button } from '../../../defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../defguard-ui/components/Layout/Button/types';
import { LabeledCheckbox } from '../../../defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { Modal } from '../../../defguard-ui/components/Layout/modals/Modal/Modal';
import { Search } from '../../../defguard-ui/components/Layout/Search/Search';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import { FilterGroupsModalFilter } from './types';

type FilterGroupDisplay = FilterGroupsModalFilter & { key: string };

type InternalStore = Record<string, Record<number, boolean>>;

type ExternalStore = Record<string, Array<number>>;

type Props = {
  isOpen: boolean;
  data: Record<string, FilterGroupsModalFilter>;
  currentState: ExternalStore;
  onCancel: () => void;
  onSubmit: (data: ExternalStore) => void;
};

export const FilterGroupsModal = ({
  data,
  isOpen,
  onCancel,
  onSubmit,
  currentState,
}: Props) => {
  return (
    <Modal
      id="acl-rules-index-filter-modal"
      className="filter-modal"
      isOpen={isOpen}
      onClose={onCancel}
    >
      <DialogContent
        data={data}
        externalState={currentState}
        onCancel={onCancel}
        onSubmit={onSubmit}
      />
    </Modal>
  );
};

type ContentProps = Pick<Props, 'onCancel' | 'onSubmit'> & {
  data: Props['data'];
  externalState: Props['currentState'];
};

const DialogContent = ({ onCancel, onSubmit, data, externalState }: ContentProps) => {
  const initialStoreState = useMemo(() => {
    const res: InternalStore = {};
    Object.entries(data).forEach(([key, value]) => {
      const items: Record<number, boolean> = {};
      value.items.forEach((item) => {
        items[item.value] = externalState[key]?.includes(item.value) ?? false;
      });
      res[key] = items;
    });
    return res;
  }, [data, externalState]);

  const initialSelectionCount = useMemo(() => {
    return flatten(Object.values(externalState)).length;
  }, [externalState]);

  const totalOptionsAvailable = useMemo(() => {
    return Object.values(data).reduce((count, filter) => filter.items.length + count, 0);
  }, [data]);
  const [searchValue, setSearch] = useState('');
  const [selected, setSelected] = useState(initialStoreState);
  const [selectionCount, setSelectionCount] = useState(initialSelectionCount);

  const { LL } = useI18nContext();

  const displayFilters = useMemo(() => {
    const res: FilterGroupDisplay[] = [];
    const isSearch = searchValue !== '';
    const clearedSearch = searchValue.trim().toLowerCase();
    const keys = Object.keys(data);

    for (const key of keys) {
      const group = { ...data[key], key };
      if (isSearch) {
        group.items = group.items.filter((item) => {
          for (const searchValue of item.searchValues) {
            if (searchValue.toLowerCase().trim().includes(clearedSearch)) {
              return true;
            }
          }
          return false;
        });
      }
      res.push(group);
    }
    return orderBy(
      res.filter((g) => g.items.length > 0),
      ['order'],
      ['asc'],
    );
  }, [data, searchValue]);

  const toggleCheckbox = useCallback(
    (groupKey: string, itemKey: number, value: boolean) => {
      setSelected((s) => ({
        ...s,
        [groupKey]: {
          ...s[groupKey],
          [itemKey]: value,
        },
      }));
      if (value) {
        setSelectionCount((s) => s + 1);
      } else {
        setSelectionCount((s) => s - 1);
      }
    },
    [],
  );

  const toggleStateAll = useCallback(
    (value: boolean, state: InternalStore) => {
      const newState = Object.fromEntries(
        Object.entries(state).map(([outerKey, innerObj]) => [
          outerKey,
          Object.fromEntries(
            Object.entries(innerObj).map(([innerKey]) => [Number(innerKey), value]),
          ),
        ]),
      );
      setSelected(newState);
      if (value) {
        setSelectionCount(totalOptionsAvailable);
      } else {
        setSelectionCount(0);
      }
    },
    [totalOptionsAvailable],
  );

  const handleSubmit = useCallback(() => {
    const res: ExternalStore = {};
    Object.entries(selected).forEach(([filterGroup, groupFilters]) => {
      const selectedFilters = Object.entries(groupFilters)
        .map(([itemId, itemSelected]) => {
          if (itemSelected) {
            return Number(itemId);
          }
        })
        .filter((id) => isPresent(id));
      res[filterGroup] = selectedFilters;
    });
    onSubmit(res);
  }, [onSubmit, selected]);

  return (
    <>
      <Search placeholder={LL.common.search()} onDebounce={setSearch} />
      <div className="controls">
        <LabeledCheckbox
          label="Select all"
          value={selectionCount === totalOptionsAvailable}
          onChange={(value) => {
            toggleStateAll(value, selected);
          }}
        />
        <button
          type="button"
          className="clear"
          onClick={() => {
            toggleStateAll(false, selected);
          }}
          disabled={selectionCount === 0}
        >
          <p>Clear all</p>
        </button>
      </div>
      <div className="groups">
        {displayFilters.map((group) => (
          <div className="group" key={group.key}>
            <GroupExpandable
              groupLabel={group.label}
              selectionCount={countSelected(selected[group.key])}
            >
              <ul className="items">
                {group.items.map((item) => (
                  <li key={item.value}>
                    <LabeledCheckbox
                      value={selected[group.key][item.value]}
                      label={item.label}
                      onChange={() => {
                        toggleCheckbox(
                          group.key,
                          item.value,
                          !selected[group.key][item.value],
                        );
                      }}
                    />
                  </li>
                ))}
              </ul>
            </GroupExpandable>
          </div>
        ))}
      </div>
      <div className="divider">
        <div className="line"></div>
      </div>
      <div className="modal-controls">
        <Button
          text={LL.common.controls.cancel()}
          size={ButtonSize.TINY}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={onCancel}
        />
        <Button
          text={`Save Filter${selectionCount !== 0 ? ` (${selectionCount})` : ''}`}
          size={ButtonSize.TINY}
          styleVariant={ButtonStyleVariant.PRIMARY}
          onClick={handleSubmit}
        />
      </div>
    </>
  );
};

const countSelected = (object: Record<number, boolean>): number => {
  let count = 0;
  for (const key in object) {
    if (object[key]) count++;
  }
  return count;
};

type GroupExpandableProps = Pick<GroupHeaderProps, 'selectionCount' | 'groupLabel'> &
  PropsWithChildren;

const GroupExpandable = ({
  groupLabel,
  selectionCount,
  children,
}: GroupExpandableProps) => {
  const [isOpen, setIsOpen] = useState(true);

  return (
    <div className={clsx('group-expandable')}>
      <GroupHeader
        groupLabel={groupLabel}
        selectionCount={selectionCount}
        arrowDirection={isOpen ? ArrowSingleDirection.DOWN : ArrowSingleDirection.UP}
        onClick={() => {
          setIsOpen((s) => !s);
        }}
      />
      <div
        className={clsx('expandable', {
          expanded: isOpen,
        })}
      >
        <div>{children}</div>
      </div>
    </div>
  );
};

type GroupHeaderProps = {
  selectionCount: number;
  groupLabel: string;
  arrowDirection: ArrowSingleDirection;
  onClick?: () => void;
};

const GroupHeader = ({
  groupLabel,
  selectionCount,
  arrowDirection,
  onClick,
}: GroupHeaderProps) => {
  const headerText = () => {
    if (selectionCount > 0) {
      return `${groupLabel} (${selectionCount})`;
    }
    return groupLabel;
  };
  return (
    <div
      className="group-header"
      onClick={() => {
        onClick?.();
      }}
    >
      <p>{headerText()}</p>
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="22"
        height="22"
        viewBox="0 0 22 22"
        fill="none"
        className={clsx({
          up: arrowDirection === ArrowSingleDirection.UP,
          down: arrowDirection === ArrowSingleDirection.DOWN,
        })}
      >
        <path
          d="M5.34276 9.75794L9.5854 14.0006C9.97592 14.3911 10.6091 14.3911 10.9996 14.0006C11.3901 13.6101 11.3901 12.9769 10.9996 12.5864L6.75697 8.34372C6.36645 7.9532 5.73328 7.9532 5.34276 8.34372C4.95223 8.73425 4.95223 9.36741 5.34276 9.75794Z"
          fill="#899CA8"
        />
        <path
          d="M11.3428 13.9994L15.5854 9.75679C15.9759 9.36627 15.9759 8.7331 15.5854 8.34258C15.1949 7.95205 14.5617 7.95205 14.1712 8.34258L9.92855 12.5852C9.53802 12.9757 9.53802 13.6089 9.92855 13.9994C10.3191 14.39 10.9522 14.39 11.3428 13.9994Z"
          fill="#899CA8"
        />
      </svg>
    </div>
  );
};
