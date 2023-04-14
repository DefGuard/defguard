import './style.scss';

import { HTMLMotionProps, motion, Variants } from 'framer-motion';
import { useEffect, useMemo, useState } from 'react';
import { debounceTime, Subject } from 'rxjs';

import { buttonsBoxShadow, ColorsRGB, inactiveBoxShadow } from '../../../constants';

interface Props {
  containerMotionProps?: HTMLMotionProps<'div'>;
  debounceTiming?: number;
  onDebounce?: (v: string) => void;
  className?: string;
  onChange?: (v: string) => void;
  initialValue?: string;
  placeholder?: string;
}
/**
 * Styled input component that can debounce it's input witch is handy when handling requests depending on user input stream
 */
export const Search = ({
  className,
  containerMotionProps,
  debounceTiming = 1000,
  onDebounce,
  onChange,
  placeholder,
  initialValue = '',
}: Props) => {
  const [inputValue, setInputValue] = useState(initialValue);
  const [focused, setFocused] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [changeSubject, setChangeSubject] = useState<Subject<string> | undefined>();

  useEffect(() => {
    if (changeSubject) {
      const sub = changeSubject.pipe(debounceTime(debounceTiming)).subscribe((value) => {
        if (onDebounce) {
          onDebounce(value);
        }
      });
      return () => sub.unsubscribe();
    } else {
      setChangeSubject(new Subject());
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [changeSubject]);

  const activeVariant = useMemo(() => {
    if (hovered && !focused) {
      return 'hover';
    }
    if (focused) {
      return 'active';
    }
    return 'idle';
  }, [focused, hovered]);

  return (
    <motion.div
      className={className ? `search ${className}` : 'search'}
      {...containerMotionProps}
    >
      <motion.input
        placeholder={placeholder}
        value={inputValue}
        initial="idle"
        variants={inputVariants}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        onHoverStart={() => setHovered(true)}
        onHoverEnd={() => setHovered(false)}
        animate={activeVariant}
        onChange={(e) => {
          if (onChange) {
            onChange(e.target.value);
          }
          if (onDebounce) {
            changeSubject?.next(e.target.value);
          }
          setInputValue(e.target.value);
        }}
      />
      <SearchIcon focus={focused} />
    </motion.div>
  );
};

const inputVariants: Variants = {
  idle: {
    backgroundColor: ColorsRGB.BgLight,
    borderColor: ColorsRGB.GrayBorder,
    boxShadow: inactiveBoxShadow,
  },
  active: {
    borderColor: ColorsRGB.GrayLighter,
    backgroundColor: ColorsRGB.White,
    boxShadow: inactiveBoxShadow,
  },
  hover: {
    backgroundColor: ColorsRGB.BgLight,
    borderColor: ColorsRGB.GrayBorder,
    boxShadow: buttonsBoxShadow,
  },
};

const searchIconVariants: Variants = {
  idle: {
    fill: '#899ca8',
  },
  active: {
    fill: ColorsRGB.Primary,
  },
};

interface SearchIconProps {
  focus: boolean;
}

const SearchIcon = ({ focus }: SearchIconProps) => {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22}>
      <defs>
        <clipPath id="icon-search_svg__a">
          <path className="icon-search_svg__a" d="M0 0h22v22H0z" />
        </clipPath>
        <style>
          {
            // eslint-disable-next-line max-len
            '\n      .icon-search_svg__a{fill:#899ca8}.icon-search_svg__a{opacity:0}.icon-search_svg__b{clip-path:url(#icon-search_svg__a)}\n    '
          }
        </style>
      </defs>
      <g className="icon-search_svg__b">
        <motion.path
          variants={searchIconVariants}
          animate={focus ? 'active' : 'idle'}
          className="icon-search_svg__c"
          // eslint-disable-next-line max-len
          d="M10.379 4a6.375 6.375 0 0 1 4.951 10.4L18 17.067l-.933.933-2.667-2.67A6.378 6.378 0 1 1 10.379 4Zm0 11.438a5.059 5.059 0 1 0-5.059-5.059 5.065 5.065 0 0 0 5.059 5.059Z"
        />
      </g>
    </svg>
  );
};
