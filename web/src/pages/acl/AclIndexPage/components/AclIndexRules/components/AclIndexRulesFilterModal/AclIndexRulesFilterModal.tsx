import './style.scss';

import { useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { LabeledCheckbox } from '../../../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Layout/modals/Modal/Modal';
import { Search } from '../../../../../../../shared/defguard-ui/components/Layout/Search/Search';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { FilterDialogFilter } from '../../types';

type FilterGroupDisplay = FilterDialogFilter & { key: string };

type InternalStore = Record<string, Record<number, boolean>>;

type ExternalStore = Record<string, Array<number>>;

type Props = {
  isOpen: boolean;
  data: Record<string, FilterDialogFilter>;
  currentState: ExternalStore;
  onCancel: () => void;
  onSubmit: (data: ExternalStore) => void;
};

export const AclIndexRulesFilterModal = ({
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

  const [searchValue, setSearch] = useState('');
  const [selected, setSelected] = useState(initialStoreState);
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
    return res;
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
    },
    [],
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
      <div className="groups">
        {displayFilters
          .filter((group) => group.items.length > 0)
          .map((group) => (
            <div className="group" key={group.key}>
              <div className="header">
                <p>{group.label}</p>
              </div>
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
            </div>
          ))}
      </div>
      <div className="divider">
        <div className="line"></div>
      </div>
      <div className="controls">
        <Button
          text={LL.common.controls.cancel()}
          size={ButtonSize.TINY}
          styleVariant={ButtonStyleVariant.STANDARD}
          onClick={onCancel}
        />
        <Button
          text={LL.common.controls.accept()}
          size={ButtonSize.TINY}
          styleVariant={ButtonStyleVariant.PRIMARY}
          onClick={handleSubmit}
        />
      </div>
    </>
  );
};
