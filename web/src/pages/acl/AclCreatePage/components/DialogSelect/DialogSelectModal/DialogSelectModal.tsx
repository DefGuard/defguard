import './style.scss';

import { ReactNode, useCallback, useEffect, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { CheckBox } from '../../../../../../shared/defguard-ui/components/Layout/Checkbox/CheckBox';
import { Modal } from '../../../../../../shared/defguard-ui/components/Layout/modals/Modal/Modal';
import { Search } from '../../../../../../shared/defguard-ui/components/Layout/Search/Search';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';
import { searchByKeys } from '../../../../../../shared/utils/searchByKeys';
import { DialogSelectProps } from '../types';

type Props<T, I> = {
  initiallySelected: I[];
  options: T[];
  getIdent: (value: T) => I;
  getLabel: (value: T) => ReactNode;
  open: boolean;
  setOpen: (val: boolean) => void;
  onChange: (selected: I[]) => void;
} & Pick<DialogSelectProps<T, I>, 'searchFn' | 'searchKeys'>;

export const DialogSelectModal = <T extends object, I extends number | string>({
  getIdent,
  initiallySelected,
  getLabel,
  open,
  setOpen,
  options,
  onChange,
  searchFn,
  searchKeys,
}: Props<T, I>) => {
  const { LL } = useI18nContext();
  const [searchValue, setSearch] = useState('');
  const [selected, setSelected] = useState<I[]>(initiallySelected);

  const handleSelect = useCallback((id: I, selected: boolean) => {
    if (selected) {
      setSelected((s) => s.filter((i) => i !== id));
    } else {
      setSelected((s) => [...s, id]);
    }
  }, []);

  const handleSelectAll = () => {
    if (selected.length === options.length) {
      setSelected([]);
    } else {
      setSelected(options.map((o) => getIdent(o)));
    }
  };

  const searchEnabled = isPresent(searchFn) || isPresent(searchKeys);

  const filteredOptions = useMemo(() => {
    if (!searchEnabled) return options;
    if (searchFn) {
      return options.filter((o) => searchFn(o, searchValue));
    }
    if (searchKeys) {
      return options.filter((o) => {
        const res = searchByKeys(o, searchKeys, searchValue);
        return res;
      });
    }
    return options;
  }, [searchEnabled, searchFn, options, searchValue, searchKeys]);

  useEffect(() => {
    setSelected(initiallySelected);
  }, [initiallySelected]);

  return (
    <Modal
      isOpen={open}
      onClose={() => {
        setOpen(false);
      }}
      afterClose={() => {
        setSearch('');
      }}
      className="modal-dialog-select"
    >
      {searchEnabled && (
        <Search
          onDebounce={(value) => {
            setSearch(value);
          }}
          placeholder="Filter/Search"
        />
      )}
      <div
        className="option"
        onClick={() => {
          handleSelectAll();
        }}
      >
        <CheckBox value={selected.length === options.length} />
        <p>Select all</p>
      </div>
      <hr />
      <ul className="options">
        {filteredOptions.length === 0 && searchValue === '' && (
          <p className="no-data">No options</p>
        )}
        {filteredOptions.length === 0 && searchValue !== '' && (
          <p className="no-data">Not found</p>
        )}
        {filteredOptions.map((o) => {
          const id = getIdent(o);
          const isSelected = selected.includes(id);
          return (
            <li
              className="option"
              key={id}
              onClick={() => {
                handleSelect(id, isSelected);
              }}
            >
              <CheckBox value={isSelected} />
              {getLabel(o)}
            </li>
          );
        })}
      </ul>
      <hr />
      <div className="controls">
        <Button
          size={ButtonSize.TINY}
          text={LL.common.controls.cancel()}
          onClick={() => {
            setOpen(false);
          }}
        />
        <Button
          size={ButtonSize.TINY}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.common.controls.accept()}
          onClick={() => {
            onChange(selected);
            setOpen(false);
          }}
        />
      </div>
    </Modal>
  );
};
