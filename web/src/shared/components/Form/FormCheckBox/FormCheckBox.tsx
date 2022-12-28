import {
  FieldValues,
  useController,
  UseControllerProps,
} from 'react-hook-form';

import { CheckBox, CheckBoxProps } from '../../layout/Checkbox/CheckBox';

interface Props<T extends FieldValues> extends Partial<CheckBoxProps> {
  controller: UseControllerProps<T>;
}

export const FormCheckBox = <T extends FieldValues>({
  controller,
  ...rest
}: Props<T>) => {
  const {
    field: { value, onChange },
  } = useController(controller);
  return <CheckBox {...rest} value={value} onChange={onChange} />;
};
