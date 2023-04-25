import './style.scss';

import { Story } from '@ladle/react';
import { isUndefined } from 'lodash-es';
import { useEffect, useMemo, useState } from 'react';

import { Input, InputFloatingErrors } from '../Input';

interface Props {
  value: string;
  disabled: boolean;
  outerLabel: string;
  disableOuterLabelColon: boolean;
  error: string;
  errors: string;
}

export const InputStory: Story<Props> = ({ value, error, errors, ...rest }) => {
  const [inputValue, setInputValue] = useState(value);

  useEffect(() => {
    setInputValue(value);
  }, [value]);

  const errorMessage = useMemo(() => {
    if (error && error.length > 0) return error;
    return undefined;
  }, [error]);

  const floatingErrors = useMemo(
    (): InputFloatingErrors => ({
      title: 'Please correct the following errors:',
      errorMessages: errors?.split(',') ?? [],
    }),
    [errors]
  );

  return (
    <Input
      errorMessage={errorMessage}
      floatingErrors={floatingErrors}
      value={inputValue}
      onChange={(e) => setInputValue(e.target.value)}
      invalid={!isUndefined(errorMessage) || floatingErrors.errorMessages.length > 0}
      {...rest}
    />
  );
};

InputStory.storyName = 'Input';
InputStory.args = {
  value: 'Test value',
  outerLabel: 'Test label',
  disabled: false,
  disableOuterLabelColon: false,
  error: 'Single error',
  errors: 'Error 1, Error 2, Error 3',
};
