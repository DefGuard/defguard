import { useMemo } from 'react';
import { FieldValues, useController, UseControllerProps } from 'react-hook-form';

import { CheckBox, CheckBoxProps } from '../../layout/Checkbox/CheckBox';

interface Props<T extends FieldValues> extends Partial<CheckBoxProps> {
  controller: UseControllerProps<T>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  customValue?: (context: any) => boolean;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  customOnChange?: (context: any) => unknown;
}

export const FormCheckBox = <T extends FieldValues>({
  controller,
  customValue,
  customOnChange,
  ...rest
}: Props<T>) => {
  const {
    field: { value, onChange },
  } = useController(controller);
  const checkBoxValue = useMemo(() => {
    if (customValue) {
      return customValue(value);
    }
    return value;
  }, [customValue, value]);
  return (
    <CheckBox
      data-testid={`field-${controller.name}`}
      {...rest}
      value={checkBoxValue}
      onChange={(e) => {
        if (customOnChange) {
          onChange(customOnChange(value));
        } else {
          onChange(e);
        }
      }}
    />
  );
};
