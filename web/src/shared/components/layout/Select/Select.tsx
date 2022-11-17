import './style.scss';

import {
  flip,
  offset,
  size,
  useFloating,
} from '@floating-ui/react-dom-interactions';
import classNames from 'classnames';
import { AnimatePresence, motion, Variants } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import { last } from 'radash';
import { Key, useEffect, useId, useMemo, useRef, useState } from 'react';
import ClickAwayListener from 'react-click-away-listener';
import { debounceTime, filter, Subject } from 'rxjs';

import {
  buttonsBoxShadow,
  ColorsRGB,
  inactiveBoxShadow,
} from '../../../constants';
import { Tag } from '../Tag/Tag';
import { SelectArrowIcon } from './SelectArrowIcon';
import { SelectOption } from './SelectOption';

export interface SelectOption<T> {
  value: T;
  label: string;
  disabled?: boolean;
  key: Key;
}

export type SelectResult<T> = SelectOption<T> | SelectOption<T>[] | undefined;

export interface SelectProps<T> {
  selected?: SelectResult<T>;
  options?: SelectOption<T>[];
  onChange: (res: SelectResult<T>) => void;
  onRemove?: (
    v: SelectOption<T>,
    selected: SelectOption<T>[]
  ) => SelectResult<T>;
  onSearch?: (value?: string) => void;
  valid?: boolean;
  invalid?: boolean;
  errorMessage?: string;
  searchMinLength?: number;
  searchDebounce?: number;
  searchable?: boolean;
  placeholder?: string;
  multi?: boolean;
  loading?: boolean;
  disabled?: boolean;
  outerLabel?: string;
  disableOuterLabelColon?: boolean;
  inForm?: boolean;
}

const defaultOnRemove = <T,>(v: SelectOption<T>, pool: SelectOption<T>[]) =>
  pool.filter((o) => o.key !== v.key);

export const Select = <T,>({
  onChange,
  onSearch,
  onRemove,
  options,
  placeholder,
  selected,
  multi,
  outerLabel,
  invalid,
  errorMessage,
  searchable = false,
  loading = false,
  disabled = false,
  searchDebounce = 1000,
  searchMinLength = 1,
  disableOuterLabelColon = false,
  inForm = false,
}: SelectProps<T>): React.ReactElement => {
  const selectId = useId();
  const [open, setOpen] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [multiSearch, setMultiSearch] = useState('');
  const multiSearchRef = useRef<HTMLInputElement | null>(null);
  const searchPushRef = useRef<HTMLSpanElement | null>(null);
  const [searchSubject] = useState<Subject<string | undefined>>(new Subject());

  const { x, y, reference, floating, strategy } = useFloating({
    open,
    onOpenChange: setOpen,
    placement: 'bottom',
    middleware: [
      offset(5),
      flip(),
      size({
        apply: ({ rects, elements }) => {
          Object.assign(elements.floating.style, {
            width: `${rects.reference.width}px`,
          });
        },
      }),
    ],
  });

  const handleSelect = (option: SelectOption<T>) => {
    if (multi) {
      if (!isUndefined(selected) && Array.isArray(selected)) {
        const existing = selected.find((o) => o.key === option.key);
        if (existing) {
          const filtered = selected.filter((o) => o.key !== option.key);
          onChange(filtered);
        } else {
          onChange([...selected, option]);
        }
      } else {
        return [option];
      }
    } else {
      onChange(option);
    }
  };

  const getClassName = useMemo(() => {
    return classNames('select', {
      disabled: loading || disabled,
      open: open,
      selected: selected,
      multi: multi,
      single: !multi,
      'in-form': inForm,
    });
  }, [disabled, inForm, loading, multi, open, selected]);

  const showSelectInnerPlaceholder = useMemo(() => {
    if (Array.isArray(selected)) {
      if (selected.length) {
        return false;
      }
    } else {
      if (selected) {
        return false;
      }
    }
    if (multiSearch) {
      return false;
    }
    return true;
  }, [multiSearch, selected]);

  const getContainerVariant = useMemo(() => {
    if (disabled) {
      return 'disabled';
    }
    if (open) {
      return 'activeOpen';
    }
    if (hovered) {
      return 'active';
    }
    return 'idle';
  }, [disabled, hovered, open]);

  const getSearchInputLength = useMemo(() => {
    const searchLength = multiSearch?.length;
    if (searchLength > 0) {
      return searchLength * 8;
    }
    if (placeholder) {
      return placeholder.length * 8 || 2;
    }
    return 2;
  }, [multiSearch?.length, placeholder]);

  const focusSearch = () => {
    if (multi && multiSearchRef) {
      multiSearchRef.current?.focus();
    }
  };

  useEffect(() => {
    const sub = searchSubject
      .pipe(
        debounceTime(searchDebounce),
        filter(
          (searchValue) =>
            !isUndefined(searchValue) && searchValue.length >= searchMinLength
        )
      )
      .subscribe((searchValue) => {
        if (onSearch) {
          onSearch(searchValue);
        }
      });
    return () => sub.unsubscribe();
  }, [onSearch, searchDebounce, searchMinLength, searchSubject]);

  return (
    <>
      {outerLabel && outerLabel.length > 0 && (
        <label className="select-outer-label" htmlFor={selectId}>
          {outerLabel}
          {!disableOuterLabelColon && ':'}
        </label>
      )}
      <motion.div
        className={getClassName}
        onClick={() => {
          if (!disabled && !loading) {
            setOpen(true);
            focusSearch();
          }
        }}
        onHoverStart={() => setHovered(true)}
        onHoverEnd={() => setHovered(false)}
        variants={selectContainerVariants}
        animate={getContainerVariant}
        ref={reference}
        id={selectId}
      >
        {!(multi || searchable) && !Array.isArray(selected) ? (
          <div className="inner-frame">
            {!selected && (
              <motion.span
                className="placeholder"
                variants={selectContainerTextVariants}
              >
                {placeholder}
              </motion.span>
            )}
            {selected && (
              <motion.span
                className="selected"
                variants={selectContainerTextVariants}
              >
                {selected.label ?? selected.value}
              </motion.span>
            )}
            <SelectArrowIcon active={open} />
          </div>
        ) : (
          <div className="inner-frame multi">
            <div className="content-frame">
              {selected &&
                Array.isArray(selected) &&
                selected.map((o) => (
                  <Tag
                    key={o.key}
                    text={o.label}
                    disposable
                    onDispose={() => {
                      if (onRemove) {
                        onChange(onRemove(o, selected));
                      } else {
                        onChange(defaultOnRemove(o, selected));
                      }
                    }}
                  />
                ))}
              <div className="multi-select-search-frame">
                <span className="input-push" ref={searchPushRef}>
                  {multiSearch &&
                    multiSearch.length &&
                    multiSearch.replace(' ', '&nbsp;')}
                  {showSelectInnerPlaceholder ? placeholder : null}
                </span>
                <input
                  className="multi-select-search"
                  value={multiSearch}
                  type="text"
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault();
                      event.stopPropagation();
                    }
                    if (event.key === 'Backspace' && multiSearch.length === 0) {
                      if (Array.isArray(selected)) {
                        const lastSelected = last(selected);
                        if (lastSelected) {
                          onChange(lastSelected);
                        }
                      }
                    }
                  }}
                  onChange={(event) => {
                    const searchValue = event.target.value;
                    setMultiSearch(searchValue);
                    searchSubject.next(searchValue);
                    if (!searchValue || searchValue.length === 0) {
                      // clear search / set to default list
                      if (onSearch) {
                        onSearch(undefined);
                      }
                    }
                  }}
                  ref={multiSearchRef}
                  placeholder={
                    showSelectInnerPlaceholder ? placeholder : undefined
                  }
                  style={{
                    width: `${getSearchInputLength}px`,
                  }}
                />
              </div>
            </div>
            <SelectArrowIcon active={open} />
          </div>
        )}
        <AnimatePresence>
          {invalid && !open && errorMessage ? (
            <motion.span
              className="error-message"
              initial={{
                x: 20,
                opacity: 0,
                bottom: 0,
              }}
              animate={{
                x: 20,
                opacity: 1,
                bottom: -20,
              }}
              exit={{
                opacity: 0,
                x: 20,
                bottom: -20,
              }}
            >
              {errorMessage}
            </motion.span>
          ) : null}
        </AnimatePresence>
      </motion.div>
      <AnimatePresence mode="wait">
        {open && (
          <ClickAwayListener
            onClickAway={() => {
              setOpen(false);
            }}
          >
            <motion.div
              initial="closed"
              animate="open"
              exit="closed"
              ref={floating}
              style={{
                position: strategy,
                left: x || 0,
                top: y || 0,
              }}
              className="select-floating-ui"
            >
              <div className="options-container">
                {options?.map((option) => {
                  const activeOption = () => {
                    if (Array.isArray(selected)) {
                      return (
                        typeof selected.find(
                          (o) => o.value === option.value
                        ) !== 'undefined'
                      );
                    }
                    return selected?.value === option.value;
                  };
                  return (
                    <SelectOption
                      key={option.key}
                      label={option.label}
                      onClick={() => {
                        handleSelect(option);
                        if (multi) {
                          focusSearch();
                        }
                      }}
                      selected={activeOption()}
                    />
                  );
                })}
              </div>
            </motion.div>
          </ClickAwayListener>
        )}
      </AnimatePresence>
    </>
  );
};

const selectContainerTextVariants: Variants = {
  idle: {
    color: ColorsRGB.TextMain,
  },
  active: {
    color: ColorsRGB.TextMain,
  },
};

const selectContainerVariants: Variants = {
  idle: {
    backgroundColor: ColorsRGB.White,
    borderColor: ColorsRGB.GrayBorder,
    boxShadow: inactiveBoxShadow,
  },
  invalidIdle: {
    backgroundColor: ColorsRGB.White,
    borderColor: ColorsRGB.Error,
    boxShadow: inactiveBoxShadow,
  },
  invalidActive: {
    backgroundColor: ColorsRGB.White,
    borderColor: ColorsRGB.Error,
    boxShadow: buttonsBoxShadow,
  },
  active: {
    backgroundColor: ColorsRGB.White,
    borderColor: ColorsRGB.GrayLighter,
    boxShadow: buttonsBoxShadow,
  },
  activeOpen: {
    backgroundColor: ColorsRGB.White,
    borderColor: ColorsRGB.GrayLighter,
    boxShadow: inactiveBoxShadow,
  },
  disabled: {
    backgroundColor: ColorsRGB.BgLight,
    borderColor: ColorsRGB.GrayBorder,
    boxShadow: inactiveBoxShadow,
    opacity: 0.6,
  },
};
