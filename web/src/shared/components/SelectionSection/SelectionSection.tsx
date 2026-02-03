import { useCallback, useMemo, useState } from 'react';
import './style.scss';
import clsx from 'clsx';
import { m } from '../../../paraglide/messages';
import { Checkbox } from '../../defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { EmptyState } from '../../defguard-ui/components/EmptyState/EmptyState';
import { Search } from '../../defguard-ui/components/Search/Search';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { sortByLabel } from '../../defguard-ui/utils/sortByLabel';
import type { SelectionKey, SelectionOption, SelectionSectionProps } from './type';

//TODO: virtualize items
export const SelectionSection = <T extends SelectionKey, M = unknown>({
  onChange,
  options,
  selection,
  className,
  id,
  renderItem,
  orderItems,
  enableDividers = false,
  itemGap = 8,
  itemHeight = 24,
}: SelectionSectionProps<T, M>) => {
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
    if (isPresent(orderItems)) return orderItems(res);
    return sortByLabel(res, (option) => option.label);
  }, [options, onlySelected, selection, search, orderItems]);

  const handleSelectAll = useCallback(() => {
    const allSelected = selection.size === options.length;
    if (allSelected) {
      if (onlySelected) {
        setOnlySelected(false);
      }
      onChange(new Set());
    } else {
      const all = options.map((o) => o.id);
      onChange(new Set(all));
    }
  }, [selection.size, onChange, options, onlySelected]);

  const handleSelect = useCallback(
    (option: SelectionOption<T>, selected: boolean, selection: Set<T>) => {
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

  const maxHeight = useMemo(() => {
    let res = itemHeight * 10;
    // add gaps
    if (enableDividers) {
      res += (itemGap * 2 + 1) * 9;
    } else {
      res += itemGap * 9;
    }
    return res;
  }, [itemGap, itemHeight, enableDividers]);

  return (
    <div className={clsx('selection-section', className)} id={id}>
      <Search
        placeholder={m.cmp_selection_section_search_placeholder()}
        initialValue={search}
        onChange={setSearch}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="actions">
        <Checkbox
          text={m.cmp_selection_section_all()}
          active={selection.size === options.length}
          onClick={handleSelectAll}
        />
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
              rowGap: enableDividers ? 0 : itemGap,
            }}
          >
            {visibleOptions.map((option, index) => {
              const isLast = index === visibleOptions.length - 1;
              const selected = selection.has(option.id);
              const handleClick = () => {
                handleSelect(option, selected, selection);
              };
              return (
                <>
                  <div
                    className="item"
                    key={option.id}
                    style={{
                      minHeight: itemHeight,
                    }}
                  >
                    {!isPresent(renderItem) && (
                      <Checkbox
                        active={selected}
                        text={option.label}
                        onClick={handleClick}
                      />
                    )}
                    {isPresent(renderItem) &&
                      renderItem({
                        option,
                        active: selected,
                        onClick: handleClick,
                      })}
                  </div>
                  {!isLast && enableDividers && (
                    <>
                      <SizedBox height={itemGap} />
                      <Divider />
                      <SizedBox height={itemGap} />
                    </>
                  )}
                </>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
