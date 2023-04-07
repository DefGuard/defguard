import React, { useMemo } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ImportConfig } from '../../../../shared/components/svg';
import SvgIconInfoNormal from '../../../../shared/components/svg/IconInfoNormal';
import { deviceBreakpoints } from '../../../../shared/constants';
import NetworkSelectButton from './NetworkSelectButton';

interface Props extends React.HTMLAttributes<HTMLInputElement> {
  onChange: (value: unknown) => void;
  value: 'import' | 'manual';
}

// eslint-disable-next-line react/display-name
const Import = React.forwardRef<HTMLInputElement, Props>(
  ({ onChange, value, ...props }, ref) => {
    const { LL } = useI18nContext();
    const { breakpoint } = useBreakpoint(deviceBreakpoints);

    const getClassName = useMemo(() => {
      const res = ['mesh-network'];
      if (value === 'import') {
        res.push('active');
      }
      if (value === 'manual') {
        res.push('not-active');
      }
      return res.join(' ');
    }, [value]);

    const handleChange = () => {
      onChange('import');
    };

    return (
      <div className={getClassName}>
        {breakpoint !== 'desktop' && <SvgIconInfoNormal />}
        <h3>{LL.wizard.wizardType.import.title()}</h3>
        {breakpoint === 'desktop' && <p>{LL.wizard.wizardType.import.description()}</p>}
        {breakpoint === 'desktop' && <ImportConfig />}
        <NetworkSelectButton
          active={typeof value !== 'undefined' && value === 'import'}
          onClick={() => handleChange()}
        />
        <input
          type="radio"
          ref={ref}
          {...props}
          onChange={onChange}
          checked={value === 'import'}
        />
      </div>
    );
  }
);

export default Import;
