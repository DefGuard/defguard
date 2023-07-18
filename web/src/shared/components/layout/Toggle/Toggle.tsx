import './style.scss';

import classNames from 'classnames';
import equal from 'fast-deep-equal';
import { motion, TargetAndTransition } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import { useMemo, useState } from 'react';

import { ColorsRGB } from '../../../constants';
import { CheckBox } from '../Checkbox/CheckBox';

export interface ToggleOption<T> {
  text: string;
  disabled?: boolean;
  value: T;
}

export interface ToggleOptionProps<T> extends ToggleOption<T> {
  onClick: () => void;
  active: boolean;
}

export interface ToggleProps<T> {
  selected: T | T[];
  options: ToggleOption<T>[];
  onChange: (v: T) => void;
  disabled?: boolean;
}

export const Toggle = <T,>({
  selected,
  options,
  onChange,
  disabled = false,
}: ToggleProps<T>) => {
  const activeOptions = useMemo((): number[] => {
    const checkEqual = (first: T, second: T): boolean => {
      if (typeof first == 'object' || Array.isArray(first)) {
        return equal(first, second);
      } else {
        return first === second;
      }
    };
    if (Array.isArray(selected)) {
      return options
        .map((optionItem, index) => {
          if (
            !isUndefined(
              selected.find((selectedItem) => checkEqual(selectedItem, optionItem.value)),
            )
          ) {
            return index;
          }
          return undefined;
        })
        .filter((index) => !isUndefined(index)) as number[];
    } else {
      return [options.findIndex((option) => checkEqual(option.value, selected))];
    }
  }, [options, selected]);

  const cn = useMemo(
    () =>
      classNames('toggle', {
        disabled,
      }),
    [disabled],
  );
  return (
    <motion.div className={cn}>
      {options.map((o, index) => (
        <ToggleOption
          {...o}
          key={index}
          active={activeOptions.includes(index)}
          onClick={() => onChange(o.value)}
        />
      ))}
    </motion.div>
  );
};

const ToggleOption = <T,>({
  text,
  onClick,
  active,
  disabled = false,
}: ToggleOptionProps<T>) => {
  const [hovered, setHovered] = useState(false);
  const cn = useMemo(
    () =>
      classNames('toggle-option', {
        active,
        disabled,
      }),
    [active, disabled],
  );

  const getAnimate = useMemo((): TargetAndTransition => {
    const res: TargetAndTransition = {
      opacity: 1,
      color: ColorsRGB.GrayLight,
      borderColor: ColorsRGB.GrayBorder,
    };

    if (disabled) {
      res.opacity = 0.5;
    }

    if (active) {
      res.color = ColorsRGB.TextMain;
      res.borderColor = ColorsRGB.Primary;
    }
    if (hovered && !active) {
      res.borderColor = ColorsRGB.GrayBorderDark;
    }
    return res;
  }, [active, disabled, hovered]);

  return (
    <motion.button
      className={cn}
      onClick={() => onClick()}
      disabled={disabled}
      initial={false}
      type="button"
      animate={getAnimate}
      onMouseOver={() => setHovered(true)}
      onMouseOut={() => setHovered(false)}
    >
      <CheckBox value={active} />
      <span>{text}</span>
    </motion.button>
  );
};
