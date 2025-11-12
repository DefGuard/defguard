import { useCallback, useMemo, useState } from 'react';
import './style.scss';
import clsx from 'clsx';
import { orderBy } from 'lodash-es';
import { m } from '../../../paraglide/messages';
import { Checkbox } from '../../defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { EmptyState } from '../../defguard-ui/components/EmptyState/EmptyState';
import { Search } from '../../defguard-ui/components/Search/Search';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing } from '../../defguard-ui/types';
import type {
  SelectionSectionKey,
  SelectionSectionOption,
  SelectionSectionProps,
} from './type';

//TODO: virtualize items
export const SelectionSection = <T extends SelectionSectionKey>({
  onChange,
  options,
  selection,
  className,
  id,
  itemGap = 8,
  itemHeight = 24,
}: SelectionSectionProps<T>) => {
  const [onlySelected, setOnlySelected] = useState(false);
  const [search, setSearch] = useState('');
  const searching = search.trim().length > 0;

  const visibleOptions = useMemo(() => {
    let res = options;
    if (onlySelected) {
      res = res.filter((o) => selection.has(o.id));
    }
    const trimmedSearch = search.trim().toLowerCase();
    if (trimmedSearch) {
      res = res.filter((option) => {
        if (option.searchFields) {
          return option.searchFields.some((val) =>
            val.toLowerCase().includes(trimmedSearch),
          );
        }
        return option.label.toLowerCase().includes(trimmedSearch);
      });
    }
    return res;
  }, [options, onlySelected, selection, search.trim]);

  const handleSelect = useCallback(
    (option: SelectionSectionOption<T>, selected: boolean, selection: Set<T>) => {
      const clone = new Set(selection);
      if (selected) {
        clone.delete(option.id);
        if (!clone.size && onlySelected) {
          setOnlySelected(false);
        }
      } else {
        clone.add(option.id);
      }
      onChange(clone);
    },
    [onChange, onlySelected],
  );

  const maxHeight = useMemo(() => itemHeight * 10 + itemGap * 9, [itemGap, itemHeight]);

  return (
    <div className={clsx('selection-section', className)} id={id}>
      <Search
        placeholder={m.cmp_selection_section_search_placeholder()}
        initialValue={search}
        onChange={setSearch}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="actions">
        <Checkbox text={m.cmp_selection_section_all()} disabled />
        <div className="right">
          <Toggle
            label={m.cmp_selection_section_selected_filter()}
            active={onlySelected}
            onClick={() => setOnlySelected((s) => !s)}
            disabled={selection.size === 0}
          />
        </div>
      </div>
      <Divider spacing={ThemeSpacing.Md} />
      {searching && visibleOptions.length === 0 && (
        <>
          <SizedBox height={130} />
          <EmptyState
            icon="search"
            title={m.cmp_selection_section_empty_title()}
            subtitle={m.cmp_selection_section_empty_subtitle()}
          />
          <SizedBox height={130} />
        </>
      )}
      {visibleOptions.length > 0 && (
        <div
          className="items-box"
          style={{
            height: maxHeight,
            maxHeight,
          }}
        >
          <div
            className="inner"
            style={{
              rowGap: itemGap,
            }}
          >
            {orderBy(
              visibleOptions,
              (item) => item.label.toLowerCase().replaceAll(' ', ''),
              ['asc'],
            ).map((option) => {
              const selected = selection.has(option.id);
              return (
                <div
                  className="item"
                  key={option.id}
                  style={{
                    minHeight: itemHeight,
                  }}
                >
                  <Checkbox
                    active={selected}
                    text={option.label}
                    onClick={() => {
                      handleSelect(option, selected, selection);
                    }}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
