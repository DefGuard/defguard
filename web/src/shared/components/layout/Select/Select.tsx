import './style.scss';

import {
  autoUpdate,
  flip,
  FloatingPortal,
  offset,
  size,
  useFloating,
} from '@floating-ui/react-dom-interactions';
import classNames from 'classnames';
import { AnimatePresence, motion, Variant, Variants } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import { last } from 'radash';
import { Key, useEffect, useId, useMemo, useRef, useState } from 'react';
import { debounceTime, filter, Subject } from 'rxjs';
import { useBreakpoint } from 'use-breakpoint';

import {
  buttonsBoxShadow,
  ColorsRGB,
  deviceBreakpoints,
  inactiveBoxShadow,
} from '../../../constants';
import { detectClickInside } from '../../../utils/detectClickOutside';
import { standardVariants } from '../../../variants';
import { LoaderSpinner } from '../LoaderSpinner/LoaderSpinner';
import { Tag } from '../Tag/Tag';
import { SelectArrowIcon } from './SelectArrowIcon';
import { SelectOption } from './SelectOption';

export type SelectValue = string | number;

export interface SelectOption<T extends SelectValue> {
  value: T;
  label: string;
  disabled?: boolean;
  key: Key;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  meta?: any;
}

export type SelectResult<T extends SelectValue> =
  | SelectOption<T>
  | SelectOption<T>[]
  | undefined;

export enum SelectStyleVariant {
  LIGHT = 'light',
  WHITE = 'white',
}

export enum SelectSizeVariant {
  STANDARD = 'STANDARD',
  SMALL = 'SMALL',
}

export interface SelectProps<T extends SelectValue> {
  selected?: SelectResult<T>;
  options?: SelectOption<T>[];
  onChange: (res: SelectResult<T>) => void;
  onRemove?: (v: SelectOption<T>, selected: SelectOption<T>[]) => SelectResult<T>;
  onSearch?: (value?: string) => void;
  onCreate?: () => void;
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
  label?: string;
  disableOuterLabelColon?: boolean;
  inForm?: boolean;
  disableOpen?: boolean;
  styleVariant?: SelectStyleVariant;
  sizeVariant?: SelectSizeVariant;
  addOptionLabel?: string;
  'data-testid'?: string;
}

const defaultOnRemove = <T extends SelectValue>(
  v: SelectOption<T>,
  pool: SelectOption<T>[],
) => pool.filter((o) => o.key !== v.key);

export const Select = <T extends SelectValue>({
  onChange,
  onSearch,
  onRemove,
  onCreate,
  options,
  placeholder,
  selected,
  label,
  invalid,
  errorMessage,
  addOptionLabel,
  multi = false,
  searchable = false,
  loading = false,
  disabled = false,
  searchDebounce = 1000,
  searchMinLength = 1,
  disableOuterLabelColon = false,
  inForm = false,
  disableOpen = false,
  styleVariant = SelectStyleVariant.LIGHT,
  sizeVariant = SelectSizeVariant.STANDARD,
  'data-testid': testId,
}: SelectProps<T>) => {
  const selectId = useId();
  const [open, setOpen] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [searchValue, setSearchValue] = useState('');
  const searchRef = useRef<HTMLInputElement | null>(null);
  const searchPushRef = useRef<HTMLSpanElement | null>(null);
  const [searchFocused, setSearchFocused] = useState(false);
  const [searchSubject] = useState<Subject<string | undefined>>(new Subject());
  const extendable = useMemo(() => !isUndefined(onCreate), [onCreate]);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const { x, y, reference, floating, strategy, refs } = useFloating({
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
    whileElementsMounted: (refElement, floatingElement, updateFunc) =>
      autoUpdate(refElement, floatingElement, updateFunc),
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
      setOpen(false);
      onChange(option);
    }
  };

  const getClassName = useMemo(() => {
    return classNames(
      'select',
      {
        disabled: loading || disabled,
        open: open,
        selected: Array.isArray(selected)
          ? selected && selected.length
          : !isUndefined(selected),
        multi: multi,
        'in-form': inForm,
      },
      `size-${sizeVariant.valueOf().toLocaleLowerCase()}`,
    );
  }, [disabled, inForm, loading, multi, open, selected, sizeVariant]);

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
    if (searchValue) {
      return false;
    }
    return true;
  }, [searchValue, selected]);

  const getContainerVariant = useMemo(() => {
    if (disabled) {
      return 'disabled';
    }
    if (open || hovered) {
      return 'active';
    }
    return 'idle';
  }, [disabled, hovered, open]);

  const getSearchInputLength = useMemo(() => {
    const searchLength = searchValue?.length;
    if (searchLength > 0) {
      return searchLength * 8;
    }
    if (placeholder) {
      return placeholder.length * 8 || 2;
    }
    return 2;
  }, [searchValue?.length, placeholder]);

  const renderTags = useMemo(() => {
    if (isUndefined(selected) && !Array.isArray(selected) && !multi) {
      return null;
    }
    if (Array.isArray(selected)) {
      return selected.map((option) => (
        <Tag
          key={option.key}
          text={option.label}
          disposable
          onDispose={() => {
            if (onRemove) {
              onChange(onRemove(option, selected));
            } else {
              onChange(defaultOnRemove(option, selected));
            }
          }}
        />
      ));
    }
  }, [multi, onChange, onRemove, selected]);

  const renderInner = useMemo(() => {
    if (searchFocused) return null;
    if (
      !searchable &&
      (!selected || (selected && Array.isArray(selected) && selected.length === 0))
    ) {
      return (
        <motion.span className="placeholder" variants={selectContainerTextVariants}>
          {placeholder}
        </motion.span>
      );
    }
    if (selected && !Array.isArray(selected) && !searchFocused) {
      return (
        <motion.span className="selected-option" variants={selectContainerTextVariants}>
          {selected.label ?? selected.value}
        </motion.span>
      );
    }
  }, [placeholder, searchFocused, searchable, selected]);

  const focusSearch = () => {
    if (searchable && searchRef && !searchFocused) {
      searchRef.current?.focus();
    }
  };

  useEffect(() => {
    const sub = searchSubject
      .pipe(
        debounceTime(searchDebounce),
        filter(
          (searchValue) =>
            !isUndefined(searchValue) && searchValue.length >= searchMinLength,
        ),
      )
      .subscribe((searchValue) => {
        if (onSearch) {
          onSearch(searchValue);
        }
      });
    return () => sub.unsubscribe();
  }, [onSearch, searchDebounce, searchMinLength, searchSubject]);

  useEffect(() => {
    const clickHandler = (env: MouseEvent) => {
      const selectRect = refs.reference.current?.getBoundingClientRect();
      const floatingRect = refs.floating.current?.getBoundingClientRect();
      if (selectRect) {
        const rects = [selectRect as DOMRect];
        if (floatingRect) {
          rects.push(floatingRect);
        }
        const clickedInside = detectClickInside(env, rects);
        if (!clickedInside) {
          setOpen(false);
        }
      }
    };
    document.addEventListener('click', clickHandler);
    return () => {
      document.removeEventListener('click', clickHandler);
    };
  }, [refs.floating, refs.reference]);

  return (
    <>
      {label && label.length > 0 && (
        <label className="select-outer-label" htmlFor={selectId}>
          {label}
          {!disableOuterLabelColon && ':'}
        </label>
      )}
      <motion.div
        className={getClassName}
        onClick={() => {
          if (open) {
            if (searchable) {
              focusSearch();
            }
          } else {
            if (!disabled && !loading && !disableOpen) {
              setOpen(true);
              if (searchable) {
                focusSearch();
              }
            }
          }
        }}
        onHoverStart={() => setHovered(true)}
        onHoverEnd={() => setHovered(false)}
        variants={selectContainerVariants}
        animate={getContainerVariant}
        custom={{
          invalid: invalid,
          styleVariant,
        }}
        ref={reference}
        id={selectId}
        data-testid={testId}
      >
        <div className="inner-frame">
          {renderInner}
          <div className="content-frame">
            {renderTags}
            {searchable && (
              <div className="search-frame">
                <span className="input-push" ref={searchPushRef}>
                  {searchValue.length > 0 && searchValue.replace(' ', '&nbsp;')}
                  {showSelectInnerPlaceholder ? placeholder : null}
                </span>
                <input
                  type="text"
                  className="select-search"
                  value={searchValue}
                  onFocus={() => setSearchFocused(true)}
                  onBlur={() => setSearchFocused(false)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault();
                      event.stopPropagation();
                    }
                    if (multi) {
                      if (event.key === 'Backspace' && searchValue.length === 0) {
                        if (Array.isArray(selected)) {
                          const lastSelected = last(selected);
                          if (lastSelected) {
                            handleSelect(lastSelected);
                          }
                        }
                      }
                    }
                  }}
                  onChange={(event) => {
                    const searchValue = event.target.value;
                    setSearchValue(searchValue);
                    searchSubject.next(searchValue);
                    if (!searchValue || searchValue.length === 0) {
                      // clear search / set to default list
                      if (onSearch) {
                        onSearch(undefined);
                      }
                    }
                  }}
                  ref={searchRef}
                  placeholder={showSelectInnerPlaceholder ? placeholder : undefined}
                  style={{
                    width: `${getSearchInputLength}px`,
                    color: searchFocused ? ColorsRGB.TextMain : 'transparent',
                  }}
                />
              </div>
            )}
          </div>
          <div className="side">
            {loading ? <LoaderSpinner size={22} /> : <SelectArrowIcon active={open} />}
          </div>
        </div>

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
      <FloatingPortal>
        <AnimatePresence mode="wait">
          {open && options && (options.length > 0 || extendable) && (
            <motion.div
              initial="hidden"
              animate="show"
              exit="hidden"
              ref={floating}
              style={{
                position: strategy,
                left: x || 0,
                top: y || 0,
              }}
              variants={standardVariants}
              transition={{
                duration: 0.08,
              }}
              className="select-floating-ui"
            >
              <div className="options-container">
                {extendable && breakpoint !== 'desktop' && (
                  <SelectOption
                    label={addOptionLabel}
                    createOption
                    onClick={() => {
                      if (onCreate) {
                        onCreate();
                        setOpen(false);
                      }
                    }}
                  />
                )}
                {options?.map((option) => {
                  const activeOption = () => {
                    if (Array.isArray(selected)) {
                      return (
                        typeof selected.find((o) => o.value === option.value) !==
                        'undefined'
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
                {extendable && breakpoint === 'desktop' && (
                  <SelectOption
                    label={addOptionLabel}
                    createOption
                    onClick={() => {
                      if (onCreate) {
                        onCreate();
                        setOpen(false);
                      }
                    }}
                  />
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </FloatingPortal>
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

interface SelectContainerCustom {
  invalid?: boolean;
  styleVariant: SelectStyleVariant;
}

const selectContainerVariants: Variants = {
  idle: ({ styleVariant, invalid }: SelectContainerCustom) => {
    const res: Variant = {
      borderColor: ColorsRGB.GrayBorder,
      boxShadow: inactiveBoxShadow,
    };
    if (styleVariant === SelectStyleVariant.LIGHT) {
      res.backgroundColor = ColorsRGB.BgLight;
    } else {
      res.backgroundColor = ColorsRGB.White;
    }
    if (invalid) {
      res.borderColor = ColorsRGB.Error;
    }
    return res;
  },
  invalidActive: {
    backgroundColor: ColorsRGB.White,
    borderColor: ColorsRGB.Error,
    boxShadow: buttonsBoxShadow,
  },
  active: ({ invalid, styleVariant }: SelectContainerCustom) => {
    const res: Variant = {
      backgroundColor: ColorsRGB.White,
      borderColor: ColorsRGB.GrayLighter,
      boxShadow: buttonsBoxShadow,
    };
    if (styleVariant === SelectStyleVariant.LIGHT) {
      res.backgroundColor = ColorsRGB.BgLight;
    } else {
      res.backgroundColor = ColorsRGB.White;
    }
    if (invalid) {
      res.borderColor = ColorsRGB.Error;
    }
    return res;
  },
  disabled: {
    backgroundColor: ColorsRGB.BgLight,
    borderColor: ColorsRGB.GrayBorder,
    boxShadow: inactiveBoxShadow,
    opacity: 0.6,
  },
};
